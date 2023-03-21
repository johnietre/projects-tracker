package database

import (
	"database/sql"
	"errors"
	"fmt"
	"strconv"
	"strings"
	"sync"

	"github.com/johnietre/projects-tracker/auth"
	"github.com/johnietre/projects-tracker/graph/model"
	sqlite3 "github.com/mattn/go-sqlite3"
)

// TODO: Implement error methods where appropriate
// UserError represents an error in user action (i.e., user input)
type UserError struct {
	err error
}

func newUserError(errStr string) UserError {
	return UserError{err: errors.New(errStr)}
}

func userInputError(why string) UserError {
	return UserError{err: newInputError(why)}
}

func userErrorFrom(err error) UserError {
	return UserError{err: err}
}

// Error returns the error string
func (ue UserError) Error() string {
	return ue.err.Error()
}

type InputError struct {
	err error
}

func newInputError(why string) InputError {
	return InputError{err: fmt.Errorf("invalid input: %s", why)}
}

func (ie InputError) Error() string {
	return ie.err.Error()
}

var (
	// ErrInvalidCreds represents invalid credentials.
	ErrInvalidCreds = newUserError("invalid credentials")
	// ErrInvalidID represents an invalid (malformed) id given.
	// It doesn't mean the ID doesn't exist.
	ErrInvalidID = userInputError("invalid ID")
	// ErrInvalidTime
	ErrInvalidTime = userInputError("invalid time format")
	// ErrPartNotExist represents a non-existent part.
	ErrPartNotExist = newUserError("part does not exist")
	// ErrUserExist represents an trying to create an existing user.
	ErrUserExist = newUserError("user already exists")
	// ErrUserNotExist represents trying to access info for a non-existent user.
	ErrUserNotExist = newUserError("user does not exist")
)

// DB is the application database
type DB struct {
	db  *sql.DB
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
  FOREIGN KEY(parent_id) REFERENCES [%[1]s](part_id) ON DELETE CASCADE
);
`

// CreateUser creates a new user with the given email/password input.
// Returns ErrUserExist if a user with the given email already exists.
// If the error is not that, it means there was an error creating the user.
func (db *DB) CreateUser(input model.CreateUserInput) (auth.User, error) {
	// TODO: Check email
	if input.Email == "" {
		return auth.User{}, userInputError("invalid email")
	}
	if l := len(input.Password); l == 0 || l > 72 {
		return auth.User{}, userInputError("invalid password")
	}
	user, err := auth.NewUser(input.Email, input.Password)
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
	if input.Email == "" || input.Password == "" {
		return auth.User{}, userInputError("invalid email or password")
	}

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
	part := &model.Part{
		Name:        input.Name,
		Description: input.Description,
		Deadline:    input.Deadline,
		CompletedAt: input.CompletedAt,
		ParentID:    input.ParentID,
	}
	dbPart, err := DBPartFromPart(part)
	if err != nil {
		return nil, err
	}

	db.mtx.Lock()
	defer db.mtx.Unlock()
	res, err := db.db.Exec(
		fmt.Sprintf(
			`INSERT INTO [%s](name,description,deadline,completed_at,parent_id) VALUES (?,?,?,?,?)`,
			email,
		),
		dbPart.Name, dbPart.Description, dbPart.Deadline, dbPart.CompletedAt, dbPart.ParentID,
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
	id, err := res.LastInsertId()
	if err == nil {
		part.ID = strconv.FormatInt(id, 10)
	}
	// TODO: do or don't return part on error?
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
  fields, vals := make([]string, 0, len(changes)), make([]any, 0, len(changes))
	for fieldName, iVal := range changes {
		valStr, ok := iVal.(string)
		if !ok {
			return nil, userInputError("malformed changes input")
		}
		var val any
		switch fieldName {
		case "name":
			if strings.TrimSpace(valStr) == "" {
				return nil, userInputError("must provide a name if changing")
			}
			val, fields = valStr, append(fields, "name=?")
		case "description":
			if valStr == "" {
				val = (*string)(nil)
			} else {
				val = &valStr
			}
      fields = append(fields, "description=?")
		case "deadline":
			ptr, err := getTimePtr(&valStr)
      if err != nil {
        return nil, ErrInvalidTime
			}
			val, fields = ptr, append(fields, "deadline=?")
		case "completed_at":
			ptr, err := getTimePtr(&valStr)
			if err != nil {
				return nil, ErrInvalidTime
			}
			val, fields = ptr, append(fields, "completed_at=?")
		default:
			return nil, userInputError(fmt.Sprintf("invalid field: %s", fieldName))
		}
		vals = append(vals, val)
	}
  setStr := strings.Join(fields, ",")

	db.mtx.Lock()
	defer db.mtx.Unlock()
	res, err := db.db.Exec(
		fmt.Sprintf(`UPDATE [%s] SET %s WHERE part_id=%d`, email, setStr, id),
		vals...,
	)
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
	id, err := strToEpoch(idStr)
	if err != nil {
		return ErrInvalidID
	}

	db.mtx.Lock()
	defer db.mtx.Unlock()
	// TODO: Get affected rows?
	_, err = db.db.Exec(fmt.Sprintf(`DELETE FROM [%s] WHERE part_id=?`, email), id)
	return err
}

// GetPart gets a single part with the given email and id.
func (db *DB) GetPart(email, idStr string) (*model.Part, error) {
	id, err := strToEpoch(idStr)
	if err != nil {
		return nil, ErrInvalidID
	}

	db.mtx.RLock()
	defer db.mtx.RUnlock()
	return db.lockedGetPart(email, id)
}

func (db *DB) lockedGetPart(email string, id int64) (*model.Part, error) {
	row := db.db.QueryRow(
		fmt.Sprintf(
			`SELECT name,description,deadline,completed_at,parent_id FROM [%s] WHERE part_id=%d`,
			email, id,
		),
	)
  dbPart := &DBPart{ID: id}
	err := row.Scan(&dbPart.Name, &dbPart.Description, &dbPart.Deadline, &dbPart.CompletedAt, &dbPart.ParentID)
	if err != nil {
		return nil, err
	}
	return dbPart.ToPart(), nil
}

// GetParts gets all the parts for the given email.
// This function will go through all the rows, even if an error occurs while
// querying a row, thus, may return the first encountered error along with
// partial rows.
func (db *DB) GetParts(email string) ([]*model.Part, error) {
	db.mtx.RLock()
	defer db.mtx.RUnlock()
	rows, err := db.db.Query(fmt.Sprintf(`SELECT * FROM [%s]`, email))
	if err != nil {
		fmt.Println(err)
		if errIsNoTable(err) {
			// TODO: Return different error since this case shouldn't happen?
			err = ErrUserNotExist
		}
		return nil, err
	}
	defer rows.Close()
	var parts []*model.Part
	for rows.Next() {
		dbPart := &DBPart{}
		e := rows.Scan(
			&dbPart.ID, &dbPart.Name, &dbPart.Description,
			&dbPart.Deadline, &dbPart.CompletedAt, &dbPart.ParentID,
		)
		if e != nil {
			fmt.Println(e)
			if err == nil {
				err = e
			}
		} else {
			parts = append(parts, dbPart.ToPart())
		}
	}
	return parts, err
}

// DBPart is the database (true) representation of Part
type DBPart struct {
	ID int64
	// Name is the name of the part
	Name string
	// Description is the optional description of the part
	Description *string
	// Deadline is the optional deadline of the part as a unix epoch
	Deadline *int64
	// CompletedAt is the optional time of completion of the part as a unix epoch
	CompletedAt *int64
	// ParentID is the optional ID of the parent part
	ParentID *int64
}

func DBPartFromPart(part *model.Part) (*DBPart, error) {
	dbPart := &DBPart{}
	var err error
	if part.ID != "" {
		if dbPart.ID, err = strToEpoch(part.ID); err != nil {
			return nil, ErrInvalidID
		}
	}
	if strings.TrimSpace(part.Name) == "" {
		return nil, userInputError("invalid name")
	}
	dbPart.Name = part.Name
	dbPart.Description = part.Description
	if dbPart.Deadline, err = getTimePtr(part.Deadline); err != nil {
		return nil, ErrInvalidTime
	}
	if dbPart.CompletedAt, err = getTimePtr(part.CompletedAt); err != nil {
		return nil, ErrInvalidTime
	}
	if dbPart.ParentID, err = getTimePtr(part.ParentID); err != nil {
		return nil, ErrInvalidID
	}
	return dbPart, nil
}

func (dbPart *DBPart) ToPart() *model.Part {
	return &model.Part{
		ID:          strconv.FormatInt(dbPart.ID, 10),
		Name:        dbPart.Name,
		Description: dbPart.Description,
		Deadline:    getStrPtr(dbPart.Deadline),
		CompletedAt: getStrPtr(dbPart.CompletedAt),
		ParentID:    getStrPtr(dbPart.ParentID),
	}
}

func errIsNoTable(err error) bool {
	return strings.HasPrefix(err.Error(), "no such table")
}

func strToEpoch(tStr string) (int64, error) {
	t, err := strconv.ParseInt(tStr, 10, 64)
	if t < 0 {
		err = errors.New("negative epoch")
	}
	return t, err
}

// Also works with parent_id (all's needed is an additional 0 check)
func getTimePtr(sPtr *string) (*int64, error) {
	if sPtr == nil || *sPtr == "" {
		return nil, nil
	}
	t, err := strToEpoch(*sPtr)
	if err != nil {
		return nil, err
	}
	return &t, nil
}

// Also works with parent_id (all's needed is an additional 0 check)
func getStrPtr(tPtr *int64) *string {
	if tPtr == nil {
		return nil
	}
	s := strconv.FormatInt(*tPtr, 10)
	return &s
}
