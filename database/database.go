package database

import (
  "database/sql"
  "errors"
  "fmt"
  "strconv"
  "strings"
  "sync"

  "github.com/johnietre/projects-tracker/graph/model"
  "github.com/johnietre/projects-tracker/auth"
  sqlite3 "github.com/mattn/go-sqlite3"
)

// UserError represents an error in user action (i.e., user input)
type UserError struct {
  err error
}

func newUserError(errStr string) UserError {
  return UserError{err: errors.New(errStr)}
}

// Error returns the error string
func (ue UserError) Error() string {
  return ue.err.Error()
}

var (
  // ErrInvalidCreds represents invalid credentials.
  ErrInvalidCreds = newUserError("invalid credentials")
  // ErrPartNotExist represents a non-existent part.
  ErrPartNotExist = newUserError("part does not exist")
  // ErrUserExist represents an trying to create an existing user.
  ErrUserExist = newUserError("user already exists")
  // ErrUserNotExist represents trying to access info for a non-existent user.
  ErrUserNotExist = newUserError("user does not exist")
)

// DB is the application database
type DB struct {
  db *sql.DB
  mtx sync.RWMutex
}

// NewDB opens a new SQLite database from the given path
func NewDB(path string) (*DB, error) {
  db, err := sql.Open("sqlite3", "file:"+path+"?_fk=ON")
  if err != nil {
    return nil, err
  }
  _, err = db.Exec(`CREATE TABLE IF NOT EXISTS users (
    email TEXT PRIMARY KEY,
    password_hash TEXT NOT NULL
  )`)
  if err != nil {
    db.Close()
    return nil, err
  }
  return &DB{db: db}, nil
}

// Close closes the database connection
func (db *DB) Close() error {
  db.mtx.Lock()
  defer db.mtx.Unlock()
  return db.db.Close()
}

const createTableSql = `
INSERT INTO users VALUES (?, ?);
CREATE TABLE [%[1]s] (
  part_id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  deadline TEXT,
  completed_at TEXT,
  parent_id INTEGER,
  FOREIGN KEY(parent_id) REFERENCES [%[1]s](part_id)
);
`

// CreateUser creates a new user with the given email/password input.
// Returns ErrUserExist if a user with the given email already exists.
// If the error is not that, it means there was an error creating the user.
func (db *DB) CreateUser(input model.CreateUserInput) (auth.User, error) {
  user, err := auth.NewUser(input.Email, input.Password)
  // TODO: Check for password too long
  if err != nil {
    return auth.User{}, err
  }
  db.mtx.Lock()
  defer db.mtx.Unlock()
  _, err = db.db.Exec(
    fmt.Sprintf(createTableSql, user.Email),
    input.Email, user.PasswordHash,
  )
  if err != nil {
    if errors.Is(err, sqlite3.ErrConstraintUnique) {
      err = ErrUserExist
    }
    return auth.User{}, err
  }
  return user, nil
}

// LoginUser checks the given credentials to see if the user can log in.
// Returns ErrInvalidCredentials if the credentials do not match.
// If the error is not that, it means there was an error checking the credentials.
func (db *DB) LoginUser(input model.LoginUserInput) (auth.User, error) {
  db.mtx.RLock()
  defer db.mtx.RUnlock()
  row := db.db.QueryRow(
    `SELECT password_hash FROM users WHERE email=?`, input.Email,
  )
  wantedHash := "P"
  if err := row.Scan(&wantedHash); err != nil {
    if errors.Is(err, sql.ErrNoRows) {
      err = ErrInvalidCreds
    }
    return auth.User{}, err
  }
  if !auth.CheckHash(input.Password, wantedHash) {
    return auth.User{}, ErrInvalidCreds
  }
  return auth.User{Email: input.Email}, nil
}

// CreatePart creates a new part for the given email
func (db *DB) CreatePart(email string, input model.CreatePartInput) (*model.Part, error) {
  db.mtx.Lock()
  defer db.mtx.Unlock()
  parentID, err := strconv.ParseInt(getOrDefault(input.ParentID), 10, 64)
  if err != nil {
    // NOTE: Could not check and leave as 0 since autoincrement should start
    // at 1 (i.e., there should be nothing with id = 0)
    parentID = -1
  }
  res, err := db.db.Exec(
    fmt.Sprintf(
      `INSERT INTO [%s](name,description,deadline,completed_at,parent_id) VALUES (?,?,?,?)`,
      email,
    ),
    input.Name, input.Description, input.Deadline, input.CompletedAt, parentID,
  )
  if err != nil {
    if errors.Is(err, sqlite3.ErrConstraintUnique) {
      err = ErrPartNotExist
    } else if errIsNoTable(err) {
      // TODO: Return different error since this case shouldn't happen?
      err = ErrUserNotExist
    }
    return nil, err
  }
  part := &model.Part{
    Name: input.Name,
    Description: input.Description,
    Deadline: input.Deadline,
    CompletedAt: input.CompletedAt,
    ParentID: input.ParentID,
  }
  id, err := res.LastInsertId()
  if err == nil {
    part.ID = strconv.FormatInt(id, 10)
  }
  // TODO: don't return part on error?
  return part, err
}

// UpdatePart updates a part with the given id for the given string with the
// given map of changes.
func (db *DB) UpdatePart(email string, idStr string, changes map[string]any) (*model.Part, error) {
  if len(changes) == 0 {
    return db.GetPart(email, idStr)
  }
  id, err := strconv.ParseInt(idStr, 10, 64)
  if err != nil {
    return nil, ErrPartNotExist
  }
  setStr, vals := "", make([]any, 0, len(changes))
  for fieldName, val := range changes {
    switch fieldName {
    case "name":
      setStr += " name=?"
    case "description":
      setStr += " description=?"
    case "deadline":
      setStr += " deadline=?"
    case "completed_at":
      setStr += " completed_at=?"
    }
    vals = append(vals, val)
  }
  stmt := fmt.Sprintf(`UPDATE [%s] SET %s WHERE id=%d`, email, setStr, id)
  db.mtx.Lock()
  defer db.mtx.Unlock()
  res, err := db.db.Exec(stmt, vals...)
  if err != nil {
    if errIsNoTable(err) {
      err = ErrUserNotExist
    }
    return nil, err
  }
  if numRows, err := res.RowsAffected(); err != nil {
    return nil, err
  } else if numRows == 0 {
    return nil, ErrPartNotExist
  }
  // TODO: Don't query and just return a part with the diffs?
  return db.lockedGetPart(email, id)
}

// DeletePart deletes a part and all its children
func (db *DB) DeletePart(email, idStr string) error {
  db.mtx.Lock()
  defer db.mtx.Unlock()
  _, err := db.db.Exec(``)
  return nil
}

// GetPart gets a single part with the given email and id.
func (db *DB) GetPart(email, idStr string) (*model.Part, error) {
  id, err := strconv.ParseInt(idStr, 10, 64)
  if err != nil {
    return nil, ErrPartNotExist
  }
  db.mtx.RLock()
  defer db.mtx.RLock()
  return db.lockedGetPart(email, id)
}

func (db *DB) lockedGetPart(email string, id int64) (*model.Part, error) {
  part, parentID := &model.Part{ID: strconv.FormatInt(id, 10)}, int64(0)
  row := db.db.QueryRow(
    fmt.Sprintf(
      `SELECT name,description,deadline,completed_at,parent_id FROM [%s] WHERE id=%d`,
      email, id,
    ),
  )
  err := row.Scan(&part.Name, &part.Description, &part.Deadline, &part.CompletedAt, &parentID)
  if err != nil {
    return nil, err
  }
  if parentID != 0 {
    part.ParentID = new(string)
    *part.ParentID = strconv.FormatInt(parentID, 10)
  }
  return part, nil
}

// GetParts gets all the parts for the given email.
// This function will go through all the rows, even if an error occurs while
// querying a row, thus, may return the first encountered error along with
// partial rows.
func (db *DB) GetParts(email string) ([]*model.Part, error) {
  db.mtx.RLock()
  defer db.mtx.RLock()
  rows, err := db.db.Query(fmt.Sprintf(`SELECT * FROM [%s]`, email))
  if err != nil {
    if errIsNoTable(err) {
      // TODO: Return different error since this case shouldn't happen?
      err = ErrUserNotExist
    }
    return nil, err
  }
  defer rows.Close()
  var parts []*model.Part
  for rows.Next() {
    part, id, parentID := &model.Part{}, int64(0), new(int64)
    e := rows.Scan(
      &id, &part.Name, &part.Description,
      &part.Deadline, &part.CompletedAt, &parentID,
    )
    if e != nil {
      fmt.Println(e)
      if err == nil {
        err = e
      }
    } else {
      part.ID = strconv.FormatInt(id, 10)
      if parentID != nil {
        part.ParentID = new(string)
        *part.ParentID = strconv.FormatInt(*parentID, 10)
      }
      parts = append(parts, part)
    }
  }
  return parts, err
}

func getOrDefault[T any](ptr *T) (res T) {
  if ptr != nil {
    res = *ptr
  }
  return
}

func errIsNoTable(err error) bool {
  return strings.HasPrefix(err.Error(), "no such table")
}
