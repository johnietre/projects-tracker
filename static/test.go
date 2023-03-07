package main
import "sync"
func main() {
  m := &sync.Mutex{}
  f(m)
  m.Unlock()
}

func f(m *sync.Mutex) {
  m.Lock()
}
