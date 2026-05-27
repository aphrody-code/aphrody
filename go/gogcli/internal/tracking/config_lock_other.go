//go:build !windows

package tracking

func isTransientTrackingConfigLockError(error) bool {
	return false
}
