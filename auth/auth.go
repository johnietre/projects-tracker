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

type ctxWriterKey string

const writerKey ctxWriterKey = "writerKey"

// User is a struct that holds user information.
type User struct {
	Email, PasswordHash string
}

type Claims struct {
	jwt.RegisteredClaims
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
	claims, ok := tok.Claims.(*Claims)
	// TODO: Is token.Valid necessary?
	//if !ok || !tok.Valid {
	if !ok {
		return User{}, false
	}
	email := claims.Subject
	if email == "" {
		return User{}, false
	}
	return User{Email: email}, true
}

func writerFromContext(ctx context.Context) (http.ResponseWriter, bool) {
	// TODO: Return specific error?
	iWriter := ctx.Value(writerKey)
	if iWriter == nil {
		return nil, false
	}
	writer, ok := iWriter.(http.ResponseWriter)
	return writer, ok
}

const (
	cookieName = "projects-tracker-tok"
	sameSite   = http.SameSiteStrictMode
)

func AddCookieToContext(ctx context.Context, jwt string) bool {
	w, ok := writerFromContext(ctx)
	if !ok {
		return false
	}
	http.SetCookie(w, &http.Cookie{
		Name:     cookieName,
		Value:    jwt,
		SameSite: sameSite,
	})
	return true
}

func RemoveCookieFromContext(ctx context.Context) bool {
	w, ok := writerFromContext(ctx)
	if !ok {
		return false
	}
	http.SetCookie(w, &http.Cookie{
		Name:     cookieName,
		Value:    "",
		SameSite: sameSite,
		MaxAge:   -1,
	})
	return true
}

// CheckHash checks to see if the given password and hashed password match.
func CheckHash(password, hash string) bool {
	return bcrypt.CompareHashAndPassword([]byte(hash), []byte(password)) == nil
}

// Middleware returns middleware that extracts a user, if it exists
func Middleware(h http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		r = r.WithContext(context.WithValue(r.Context(), writerKey, w))
		tokStr := ""
		cookie, err := r.Cookie(cookieName)
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

// TestMiddleware returns middleware for testing
func TestMiddleware(h http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		user := User{Email: "johnietrebus@gmail.com"}
		r = r.WithContext(context.WithValue(r.Context(), userKey, user))
		h.ServeHTTP(w, r)
	})
}

var jwtKey = []byte("611d23ea-4292-49da-a45a-0f1df6a69152")

// GenerateToken generates a new JWT for the given email
func GenerateToken(email string) (string, error) {
	now := time.Now()
	claims := &Claims{
		jwt.RegisteredClaims{
			Subject:   email,
			ExpiresAt: jwt.NewNumericDate(now.Add(time.Hour * 24 * 365)),
			IssuedAt:  jwt.NewNumericDate(now),
		},
	}
	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	return token.SignedString(jwtKey)
}

func parseToken(tokStr string) (tok *jwt.Token, err error) {
	claims := &Claims{}
	defer func() {
		if tok != nil {
			tok.Claims = claims
		}
	}()
	return jwt.ParseWithClaims(tokStr, claims, func(tok *jwt.Token) (any, error) {
		if _, ok := tok.Method.(*jwt.SigningMethodHMAC); !ok {
			return nil, fmt.Errorf("Unexpected signing method: %v", tok.Header["alg"])
		}
		return jwtKey, nil
	})
}
