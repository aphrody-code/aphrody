//nolint:wsl_v5
package oauthclient

import (
	"errors"
	"fmt"
	"strings"

	"github.com/99designs/keyring"

	"github.com/steipete/gogcli/internal/config"
	"github.com/steipete/gogcli/internal/secrets"
)

const clientSecretKeyFmt = "client/%s/client-secret"

var (
	errEmptyClientSecret = errors.New("OAuth client secret in keyring is empty")
	setSecret            = secrets.SetSecret
	getSecret            = secrets.GetSecret
	deleteSecret         = secrets.DeleteSecret
)

func ClientSecretKey(client string) (string, error) {
	normalized, err := config.NormalizeClientNameOrDefault(client)
	if err != nil {
		return "", fmt.Errorf("normalize client: %w", err)
	}
	return fmt.Sprintf(clientSecretKeyFmt, normalized), nil
}

func WriteClientCredentialsFor(client string, creds config.ClientCredentials, insecure bool) error {
	normalized, err := config.NormalizeClientNameOrDefault(client)
	if err != nil {
		return fmt.Errorf("normalize client: %w", err)
	}
	if insecure {
		if writeErr := config.WriteClientCredentialsFor(normalized, creds); writeErr != nil {
			return fmt.Errorf("write legacy credentials: %w", writeErr)
		}
		key, keyErr := ClientSecretKey(normalized)
		if keyErr != nil {
			return keyErr
		}
		if deleteErr := deleteSecret(key); deleteErr != nil && !errors.Is(deleteErr, keyring.ErrKeyNotFound) {
			return fmt.Errorf("delete OAuth client secret from keyring: %w", deleteErr)
		}
		return nil
	}
	key, err := ClientSecretKey(normalized)
	if err != nil {
		return err
	}
	if err := setSecret(key, []byte(strings.TrimSpace(creds.ClientSecret))); err != nil {
		return fmt.Errorf("store OAuth client secret: %w", err)
	}
	if writeErr := config.WriteClientCredentialsMetadataFor(normalized, creds); writeErr != nil {
		return fmt.Errorf("write credentials metadata: %w", writeErr)
	}
	return nil
}

func ReadClientCredentialsFor(client string) (config.ClientCredentials, error) {
	normalized, err := config.NormalizeClientNameOrDefault(client)
	if err != nil {
		return config.ClientCredentials{}, fmt.Errorf("normalize client: %w", err)
	}
	creds, err := config.ReadClientCredentialsMetadataFor(normalized)
	if err != nil {
		return config.ClientCredentials{}, fmt.Errorf("read credentials metadata: %w", err)
	}
	if strings.TrimSpace(creds.ClientSecret) != "" {
		return creds, nil
	}
	key, err := ClientSecretKey(normalized)
	if err != nil {
		return config.ClientCredentials{}, err
	}
	secret, err := getSecret(key)
	if err != nil {
		return config.ClientCredentials{}, fmt.Errorf("read OAuth client secret from keyring: %w", err)
	}
	creds.ClientSecret = strings.TrimSpace(string(secret))
	if creds.ClientSecret == "" {
		return config.ClientCredentials{}, errEmptyClientSecret
	}
	return creds, nil
}

func DeleteClientCredentialsFor(client string) error {
	normalized, err := config.NormalizeClientNameOrDefault(client)
	if err != nil {
		return fmt.Errorf("normalize client: %w", err)
	}
	key, keyErr := ClientSecretKey(normalized)
	if keyErr != nil {
		return keyErr
	}

	creds, readErr := config.ReadClientCredentialsMetadataFor(normalized)
	hasPlaintextSecret := readErr == nil && strings.TrimSpace(creds.ClientSecret) != ""
	secretErr := deleteSecret(key)
	if secretErr != nil {
		if !errors.Is(secretErr, keyring.ErrKeyNotFound) && !hasPlaintextSecret {
			return fmt.Errorf("delete OAuth client secret: %w", secretErr)
		}
	}

	fileErr := config.DeleteClientCredentialsFor(normalized)
	if fileErr != nil {
		return fmt.Errorf("delete credentials metadata: %w", fileErr)
	}
	return nil
}

func ClientSecretInKeyring(client string) bool {
	key, err := ClientSecretKey(client)
	if err != nil {
		return false
	}
	secret, err := getSecret(key)
	return err == nil && strings.TrimSpace(string(secret)) != ""
}
