//go:build windows

package tracking

import (
	"errors"
	"syscall"
)

const windowsErrorSharingViolation syscall.Errno = 32

func isTransientTrackingConfigLockError(err error) bool {
	return errors.Is(err, syscall.ERROR_ACCESS_DENIED) ||
		errors.Is(err, windowsErrorSharingViolation)
}
