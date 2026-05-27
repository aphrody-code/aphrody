package cmd

import (
	"context"
	"fmt"
	"os"
	"strings"

	"github.com/steipete/gogcli/internal/googleapi"
	"github.com/steipete/gogcli/internal/outfmt"
	"github.com/steipete/gogcli/internal/ui"
)

type MapsCmd struct {
	Places         MapsPlacesCmd         `cmd:"" name:"places" aliases:"place" help:"Google Maps Places API"`
	Directions     MapsDirectionsCmd     `cmd:"" name:"directions" aliases:"route" help:"Get directions between two locations"`
	Distance       MapsDistanceCmd       `cmd:"" name:"distance" aliases:"distance-matrix,matrix" help:"Get travel distance and duration matrix"`
	Geocode        MapsGeocodeCmd        `cmd:"" name:"geocode" help:"Convert an address to coordinates"`
	ReverseGeocode MapsReverseGeocodeCmd `cmd:"" name:"reverse-geocode" aliases:"reverse" help:"Convert coordinates to an address"`
}

type MapsPlacesCmd struct {
	Search  MapsPlacesSearchCmd  `cmd:"" name:"search" aliases:"find" help:"Search Places by text"`
	Details MapsPlacesDetailsCmd `cmd:"" name:"details" aliases:"get,info,show" help:"Get Place details"`
}

type MapsDirectionsCmd struct {
	Origin      string `name:"origin" help:"Origin address, place ID, or lat,lng" required:""`
	Destination string `name:"destination" help:"Destination address, place ID, or lat,lng" required:""`
	Mode        string `name:"mode" help:"Travel mode: driving|walking|bicycling|transit"`
	Language    string `name:"language" help:"BCP-47 language code"`
	Region      string `name:"region" help:"Region bias"`
}

func (c *MapsDirectionsCmd) Run(ctx context.Context, flags *RootFlags) error {
	if strings.TrimSpace(c.Origin) == "" || strings.TrimSpace(c.Destination) == "" {
		return usage("--origin and --destination are required")
	}
	client, err := newMapsClient()
	if err != nil {
		return err
	}
	resp, err := client.Directions(ctx, c.Origin, c.Destination, googleapi.MapsDirectionsOptions{
		Mode:     strings.TrimSpace(c.Mode),
		Language: strings.TrimSpace(c.Language),
		Region:   strings.TrimSpace(c.Region),
	})
	if err != nil {
		return err
	}
	return writeMapsDirections(ctx, resp)
}

type MapsDistanceCmd struct {
	Origins      string `name:"origins" help:"Comma-separated origins" required:""`
	Destinations string `name:"destinations" help:"Comma-separated destinations" required:""`
	Mode         string `name:"mode" help:"Travel mode: driving|walking|bicycling|transit"`
	Units        string `name:"units" help:"Units: metric|imperial"`
	Language     string `name:"language" help:"BCP-47 language code"`
	Region       string `name:"region" help:"Region bias"`
}

func (c *MapsDistanceCmd) Run(ctx context.Context, flags *RootFlags) error {
	origins := splitCSV(c.Origins)
	destinations := splitCSV(c.Destinations)
	if len(origins) == 0 || len(destinations) == 0 {
		return usage("--origins and --destinations are required")
	}
	client, err := newMapsClient()
	if err != nil {
		return err
	}
	resp, err := client.DistanceMatrix(ctx, origins, destinations, googleapi.MapsDistanceMatrixOptions{
		Mode:     strings.TrimSpace(c.Mode),
		Units:    strings.TrimSpace(c.Units),
		Language: strings.TrimSpace(c.Language),
		Region:   strings.TrimSpace(c.Region),
	})
	if err != nil {
		return err
	}
	return writeMapsDistance(ctx, resp)
}

type MapsGeocodeCmd struct {
	Address  []string `arg:"" name:"address" help:"Address to geocode"`
	Language string   `name:"language" help:"BCP-47 language code"`
	Region   string   `name:"region" help:"Region bias"`
}

func (c *MapsGeocodeCmd) Run(ctx context.Context, flags *RootFlags) error {
	address := strings.TrimSpace(strings.Join(c.Address, " "))
	if address == "" {
		return usage("missing address")
	}
	client, err := newMapsClient()
	if err != nil {
		return err
	}
	resp, err := client.Geocode(ctx, address, googleapi.MapsGeocodeOptions{
		Language: strings.TrimSpace(c.Language),
		Region:   strings.TrimSpace(c.Region),
	})
	if err != nil {
		return err
	}
	return writeMapsGeocode(ctx, resp)
}

type MapsReverseGeocodeCmd struct {
	Lat      string `name:"lat" help:"Latitude" required:""`
	Lng      string `name:"lng" help:"Longitude" required:""`
	Language string `name:"language" help:"BCP-47 language code"`
	Region   string `name:"region" help:"Region bias"`
}

func (c *MapsReverseGeocodeCmd) Run(ctx context.Context, flags *RootFlags) error {
	if strings.TrimSpace(c.Lat) == "" || strings.TrimSpace(c.Lng) == "" {
		return usage("--lat and --lng are required")
	}
	client, err := newMapsClient()
	if err != nil {
		return err
	}
	resp, err := client.ReverseGeocode(ctx, strings.TrimSpace(c.Lat)+","+strings.TrimSpace(c.Lng), googleapi.MapsGeocodeOptions{
		Language: strings.TrimSpace(c.Language),
		Region:   strings.TrimSpace(c.Region),
	})
	if err != nil {
		return err
	}
	return writeMapsGeocode(ctx, resp)
}

type MapsPlacesSearchCmd struct {
	Query    []string `arg:"" name:"query" help:"Text search query"`
	Language string   `name:"language" help:"BCP-47 language code"`
	Region   string   `name:"region" help:"CLDR region code"`
}

func (c *MapsPlacesSearchCmd) Run(ctx context.Context, flags *RootFlags) error {
	query := strings.TrimSpace(strings.Join(c.Query, " "))
	if query == "" {
		return usage("missing query")
	}
	client, err := newMapsPlacesClient()
	if err != nil {
		return err
	}
	opts := googleapi.PlacesLookupOptions{
		LanguageCode: strings.TrimSpace(c.Language),
		RegionCode:   strings.TrimSpace(c.Region),
	}
	place, err := client.TextSearch(ctx, query, opts)
	if err != nil {
		return err
	}
	return writeMapsPlace(ctx, place)
}

type MapsPlacesDetailsCmd struct {
	PlaceID  string `arg:"" name:"placeId" help:"Place ID (places/{id} accepted)"`
	Language string `name:"language" help:"BCP-47 language code"`
	Region   string `name:"region" help:"CLDR region code"`
}

func (c *MapsPlacesDetailsCmd) Run(ctx context.Context, flags *RootFlags) error {
	placeID := strings.TrimSpace(c.PlaceID)
	if placeID == "" {
		return usage("empty placeId")
	}
	client, err := newMapsPlacesClient()
	if err != nil {
		return err
	}
	opts := googleapi.PlacesLookupOptions{
		LanguageCode: strings.TrimSpace(c.Language),
		RegionCode:   strings.TrimSpace(c.Region),
	}
	place, err := client.Details(ctx, placeID, opts)
	if err != nil {
		return err
	}
	return writeMapsPlace(ctx, place)
}

func newMapsPlacesClient() (*googleapi.PlacesClient, error) {
	apiKey, err := placesAPIKey()
	if err != nil {
		return nil, err
	}
	return googleapi.NewPlacesClient(apiKey, googleapi.WithPlacesBaseURL(os.Getenv("GOG_PLACES_BASE_URL"))), nil
}

func newMapsClient() (*googleapi.MapsClient, error) {
	apiKey, err := placesAPIKey()
	if err != nil {
		return nil, err
	}
	return googleapi.NewMapsClient(apiKey, googleapi.WithMapsBaseURL(os.Getenv("GOG_MAPS_BASE_URL"))), nil
}

func writeMapsPlace(ctx context.Context, place *googleapi.Place) error {
	u := ui.FromContext(ctx)
	if outfmt.IsJSON(ctx) {
		return outfmt.WriteJSON(ctx, os.Stdout, map[string]any{"place": place})
	}
	if place == nil {
		u.Err().Println("No place")
		return nil
	}
	u.Out().Linef("id\t%s", place.ID)
	if strings.TrimSpace(place.Name) != "" {
		u.Out().Linef("name\t%s", place.Name)
	}
	if strings.TrimSpace(place.FormattedAddress) != "" {
		u.Out().Linef("address\t%s", place.FormattedAddress)
	}
	if strings.TrimSpace(place.GoogleMapsURI) != "" {
		u.Out().Linef("maps_uri\t%s", place.GoogleMapsURI)
	}
	return nil
}

func writeMapsDirections(ctx context.Context, resp *googleapi.MapsDirectionsResponse) error {
	u := ui.FromContext(ctx)
	if outfmt.IsJSON(ctx) {
		return outfmt.WriteJSON(ctx, os.Stdout, map[string]any{"directions": resp})
	}
	if resp == nil || len(resp.Routes) == 0 {
		u.Err().Println("No routes")
		return nil
	}
	w, flush := tableWriter(ctx)
	defer flush()
	fmt.Fprintln(w, "SUMMARY\tSTART\tEND\tDISTANCE\tDURATION")
	for _, route := range resp.Routes {
		for _, leg := range route.Legs {
			fmt.Fprintf(w, "%s\t%s\t%s\t%s\t%s\n", route.Summary, leg.StartAddress, leg.EndAddress, leg.Distance.Text, leg.Duration.Text)
		}
	}
	return nil
}

func writeMapsDistance(ctx context.Context, resp *googleapi.MapsDistanceMatrixResponse) error {
	u := ui.FromContext(ctx)
	if outfmt.IsJSON(ctx) {
		return outfmt.WriteJSON(ctx, os.Stdout, map[string]any{"distanceMatrix": resp})
	}
	if resp == nil || len(resp.Rows) == 0 {
		u.Err().Println("No distances")
		return nil
	}
	w, flush := tableWriter(ctx)
	defer flush()
	fmt.Fprintln(w, "ORIGIN\tDESTINATION\tSTATUS\tDISTANCE\tDURATION")
	for i, row := range resp.Rows {
		origin := indexString(resp.OriginAddresses, i)
		for j, element := range row.Elements {
			fmt.Fprintf(w, "%s\t%s\t%s\t%s\t%s\n", origin, indexString(resp.DestAddresses, j), element.Status, element.Distance.Text, element.Duration.Text)
		}
	}
	return nil
}

func writeMapsGeocode(ctx context.Context, resp *googleapi.MapsGeocodeResponse) error {
	u := ui.FromContext(ctx)
	if outfmt.IsJSON(ctx) {
		return outfmt.WriteJSON(ctx, os.Stdout, map[string]any{"geocode": resp})
	}
	if resp == nil || len(resp.Results) == 0 {
		u.Err().Println("No geocode results")
		return nil
	}
	w, flush := tableWriter(ctx)
	defer flush()
	fmt.Fprintln(w, "ADDRESS\tPLACE_ID\tLAT\tLNG\tLOCATION_TYPE")
	for _, result := range resp.Results {
		fmt.Fprintf(w, "%s\t%s\t%g\t%g\t%s\n",
			result.FormattedAddress,
			result.PlaceID,
			result.Geometry.Location.Lat,
			result.Geometry.Location.Lng,
			result.Geometry.LocationType,
		)
	}
	return nil
}

func indexString(values []string, index int) string {
	if index < 0 || index >= len(values) {
		return ""
	}
	return values[index]
}
