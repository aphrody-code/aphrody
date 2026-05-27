package cmd

import (
	"context"
	"fmt"
)

type CalendarAppointmentsCmd struct{}

func (c *CalendarAppointmentsCmd) Run(ctx context.Context, flags *RootFlags) error {
	return errCalendarAppointmentSchedulesUnsupported
}

var errCalendarAppointmentSchedulesUnsupported = fmt.Errorf("calendar appointment schedules are not exposed by the Google Calendar API; Events.list currently accepts eventTypes birthday, default, focusTime, fromGmail, outOfOffice, and workingLocation only")
