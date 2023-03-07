package auth

import (
  "context"
  "fmt"
  "net/http"
  "strings"
  "time"

  jwt "github.com/golang-jwt/jwt/v5"
  "golang.org/x/crypto/bcrypt"
)

type ctxUserKey string
const userKey ctxUserKey = "userKey"

// User is a struct that holds user information.
type User struct {
  Email, PasswordHash string
}

// NewUser returns a User with the given email and password hash generated
// from the given password.
func NewUser(email, password string) (User, error) {
  hash, err := bcrypt.GenerateFromPassword([]byte(password), bcrypt.DefaultCost)
  if err != nil {
    return User{}, err
  }
  return User{Email: email, PasswordHash: string(hash)}, nil
}

// UserFromContext returns the user for the given context, returning false if
// no user was gotten.
func UserFromContext(ctx context.Context) (User, bool) {
  iUser := ctx.Value(userKey)
  if iUser == nil {
    return User{}, false
  }
  user, ok := iUser.(User)
  return user, ok
}

func userFromToken(tok *jwt.Token) (User, bool) {
  claims, ok := tok.Claims.(*jwt.RegisteredClaims)
  // TODO: Is token.Valid necessary?
  if !ok || !tok.Valid {
    return User{}, false
  }
  email := claims.Subject
  if email == "" {
    return User{}, false
  }
  return User{Email: email}, true
}

// CheckHash checks to see if the given password and hashed password match.
func CheckHash(password, hash string) bool {
  return bcrypt.CompareHashAndPassword([]byte(hash), []byte(password)) == nil
}

// Middleware returns middleware that extracts a user, if it exists
func Middleware(h http.Handler) http.Handler {
  return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
    tokStr := ""
    cookie, err := r.Cookie("projects-tracker-tok")
    if err == nil {
      tokStr = cookie.Value
    }
    if tokStr == "" {
      authParts := strings.Split(r.Header.Get("Authoirzation"), " ")
      if len(authParts) == 2 {
        tokStr = authParts[1]
      }
    }
    if tokStr != "" {
      if tok, err := parseToken(tokStr); err == nil {
        if user, ok := userFromToken(tok); ok {
          r = r.WithContext(context.WithValue(r.Context(), userKey, user))
        }
      }
    }
    h.ServeHTTP(w, r)
  })
}

var jwtKey = []byte("611d23ea-4292-49da-a45a-0f1df6a69152")

// GenerateToken generates a new JWT for the given email
func GenerateToken(email string) (string, error) {
  now := time.Now()
  claims := &jwt.RegisteredClaims{
    Subject: email,
    //ExpiresAt: jwt.NewNumericDate(now.Add(time.Day * 15)),
    IssuedAt: jwt.NewNumericDate(now),
  }
  token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
  return token.SignedString(jwtKey)
}

func parseToken(tokStr string) (*jwt.Token, error) {
  return jwt.Parse(tokStr, func(tok *jwt.Token) (any, error) {
    if _, ok := tok.Method.(*jwt.SigningMethodHMAC); !ok {
      return nil, fmt.Errorf("Unexpected signing method: %v", tok.Header["alg"])
    }
    return jwtKey, nil
  })
}
