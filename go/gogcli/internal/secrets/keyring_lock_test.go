package secrets

import (
	"bufio"
	"context"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/steipete/gogcli/internal/config"
)

func TestKeyringLockBlocksConcurrentProcess(t *testing.T) {
	if os.Getenv("GOG_TEST_HOLD_KEYRING_LOCK") == "1" {
		holdKeyringLockForTest(t)
		return
	}

	path := filepath.Join(t.TempDir(), ".lock")

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	cmd := exec.CommandContext(ctx, os.Args[0], "-test.run=TestKeyringLockBlocksConcurrentProcess", "--", path)

	cmd.Env = append(os.Environ(), "GOG_TEST_HOLD_KEYRING_LOCK=1")

	stdout, err := cmd.StdoutPipe()
	if err != nil {
		t.Fatalf("StdoutPipe: %v", err)
	}

	if startErr := cmd.Start(); startErr != nil {
		t.Fatalf("start helper: %v", startErr)
	}

	defer func() {
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
	}()

	line, err := bufio.NewReader(stdout).ReadString('\n')
	if err != nil {
		t.Fatalf("read helper readiness: %v", err)
	}

	if strings.TrimSpace(line) != "ready" {
		t.Fatalf("unexpected helper output: %q", line)
	}

	lock := &keyringLock{path: path, timeout: 50 * time.Millisecond}

	err = lock.withReadLock(func() error { return nil })
	if err == nil {
		t.Fatal("expected lock timeout")
	}

	if !errors.Is(err, errKeyringTimeout) {
		t.Fatalf("expected keyring timeout, got %v", err)
	}

	if !strings.Contains(err.Error(), keyringLockTimeoutEnv) {
		t.Fatalf("expected timeout env guidance, got %v", err)
	}
}

func holdKeyringLockForTest(t *testing.T) {
	t.Helper()

	if len(os.Args) == 0 {
		t.Fatal("missing lock path")
	}

	path := os.Args[len(os.Args)-1]

	file, err := os.OpenFile(path, os.O_CREATE|os.O_RDWR, 0o600)
	if err != nil {
		t.Fatalf("open helper lock: %v", err)
	}
	defer file.Close()

	if err := lockKeyringFile(file, true); err != nil {
		t.Fatalf("helper lock: %v", err)
	}

	fmt.Println("ready")
	time.Sleep(5 * time.Second)
}

func TestKeyringStoreFileBackendConcurrentSetToken(t *testing.T) {
	home := t.TempDir()
	t.Setenv("HOME", home)
	t.Setenv("XDG_CONFIG_HOME", filepath.Join(home, "xdg-config"))
	t.Setenv(keyringBackendEnv, "file")
	t.Setenv(keyringPasswordEnv, "test-pass")

	store, err := OpenDefault()
	if err != nil {
		t.Fatalf("OpenDefault: %v", err)
	}

	keyringStore := store.(*KeyringStore)
	if keyringStore.lock == nil {
		t.Fatal("expected file-backed store lock")
	}

	const writers = 12

	var wg sync.WaitGroup

	errs := make(chan error, writers)
	for i := 0; i < writers; i++ {
		wg.Add(1)

		go func(i int) {
			defer wg.Done()

			errs <- store.SetToken(config.DefaultClientName, "user@example.com", Token{
				Subject:      fmt.Sprintf("subject-%02d", i),
				RefreshToken: fmt.Sprintf("refresh-%02d", i),
			})
		}(i)
	}

	wg.Wait()
	close(errs)

	for err := range errs {
		if err != nil {
			t.Fatalf("SetToken: %v", err)
		}
	}

	tok, err := store.GetToken(config.DefaultClientName, "user@example.com")
	if err != nil {
		t.Fatalf("GetToken: %v", err)
	}

	if tok.RefreshToken == "" || tok.Subject == "" {
		t.Fatalf("expected final token data, got %#v", tok)
	}

	tokens, err := store.ListTokens()
	if err != nil {
		t.Fatalf("ListTokens: %v", err)
	}

	if len(tokens) != 1 {
		t.Fatalf("expected one logical token after concurrent writes, got %#v", tokens)
	}
}

func TestKeyringStoreKeysHideLockFile(t *testing.T) {
	home := t.TempDir()
	t.Setenv("HOME", home)
	t.Setenv("XDG_CONFIG_HOME", filepath.Join(home, "xdg-config"))
	t.Setenv(keyringBackendEnv, "file")
	t.Setenv(keyringPasswordEnv, "test-pass")

	store, err := OpenDefault()
	if err != nil {
		t.Fatalf("OpenDefault: %v", err)
	}

	keys, err := store.Keys()
	if err != nil {
		t.Fatalf("Keys: %v", err)
	}

	if len(keys) != 0 {
		t.Fatalf("expected no public keys for empty keyring, got %#v", keys)
	}
}

func TestSetSecretFileBackendUsesSharedLock(t *testing.T) {
	home := t.TempDir()
	t.Setenv("HOME", home)
	t.Setenv("XDG_CONFIG_HOME", filepath.Join(home, "xdg-config"))
	t.Setenv(keyringBackendEnv, "file")
	t.Setenv(keyringPasswordEnv, "test-pass")

	var wg sync.WaitGroup
	errs := make(chan error, 8)

	for i := 0; i < 8; i++ {
		wg.Add(1)

		go func(i int) {
			defer wg.Done()

			errs <- SetSecret("shared/secret", []byte(fmt.Sprintf("value-%d", i)))
		}(i)
	}

	wg.Wait()
	close(errs)

	for err := range errs {
		if err != nil {
			t.Fatalf("SetSecret: %v", err)
		}
	}

	got, err := GetSecret("shared/secret")
	if err != nil {
		t.Fatalf("GetSecret: %v", err)
	}

	if !strings.HasPrefix(string(got), "value-") {
		t.Fatalf("unexpected secret value %q", string(got))
	}
}

func TestKeyringLockTimeoutEnv(t *testing.T) {
	t.Setenv(keyringLockTimeoutEnv, "25ms")

	if got := keyringLockTimeout(); got != 25*time.Millisecond {
		t.Fatalf("expected parsed timeout, got %v", got)
	}

	t.Setenv(keyringLockTimeoutEnv, "not-a-duration")

	if got := keyringLockTimeout(); got != defaultKeyringLockTimeout {
		t.Fatalf("expected default timeout for invalid env, got %v", got)
	}
}
