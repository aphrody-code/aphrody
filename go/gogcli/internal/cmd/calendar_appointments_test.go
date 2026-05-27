package cmd

import (
	"strings"
	"testing"
)

func TestCalendarAppointmentsReportsUnsupportedAPI(t *testing.T) {
	err := (&CalendarAppointmentsCmd{}).Run(newCalendarJSONContext(t), &RootFlags{Account: "a@example.com"})
	if err == nil {
		t.Fatal("expected unsupported API error")
	}
	if !strings.Contains(err.Error(), "appointment schedules are not exposed") {
		t.Fatalf("unexpected error: %v", err)
	}
}
