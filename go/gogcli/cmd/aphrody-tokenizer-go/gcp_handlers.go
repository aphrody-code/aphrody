package main

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"sync/atomic"
	"time"

	"cloud.google.com/go/bigquery"
	cloudtasks "cloud.google.com/go/cloudtasks/apiv2"
	cloudtaskspb "cloud.google.com/go/cloudtasks/apiv2/cloudtaskspb"
	"cloud.google.com/go/firestore"
	functions "cloud.google.com/go/functions/apiv2"
	"cloud.google.com/go/functions/apiv2/functionspb"
	"cloud.google.com/go/kms/apiv1"
	kmspb "cloud.google.com/go/kms/apiv1/kmspb"
	"cloud.google.com/go/logging"
	"cloud.google.com/go/logging/logadmin"
	"cloud.google.com/go/pubsub"
	run "cloud.google.com/go/run/apiv2"
	"cloud.google.com/go/run/apiv2/runpb"
	"cloud.google.com/go/secretmanager/apiv1"
	secretmanagerpb "cloud.google.com/go/secretmanager/apiv1/secretmanagerpb"
	"cloud.google.com/go/spanner"
	"cloud.google.com/go/speech/apiv2"
	speechpb "cloud.google.com/go/speech/apiv2/speechpb"
	"cloud.google.com/go/storage"
	texttospeech "cloud.google.com/go/texttospeech/apiv1"
	"cloud.google.com/go/texttospeech/apiv1/texttospeechpb"
	"cloud.google.com/go/translate/apiv3"
	translatepb "cloud.google.com/go/translate/apiv3/translatepb"
	vision "cloud.google.com/go/vision/v2/apiv1"
	visionpb "cloud.google.com/go/vision/v2/apiv1/visionpb"
	"google.golang.org/api/iterator"
)

// GCS handlers
func runGCSUpload(ctx context.Context, req Request) (Response, error) {
	if req.Bucket == "" || req.Object == "" {
		return Response{}, errors.New("bucket and object are required")
	}

	client, err := storage.NewClient(ctx)
	if err != nil {
		return Response{}, fmt.Errorf("create storage client: %w", err)
	}
	defer client.Close()

	obj := client.Bucket(req.Bucket).Object(req.Object)
	w := obj.NewWriter(ctx)
	if req.ContentType != "" {
		w.ContentType = req.ContentType
	}

	var bytesWritten int64
	if req.LocalFile != "" {
		f, err := os.Open(req.LocalFile)
		if err != nil {
			w.Close()
			return Response{}, fmt.Errorf("open local file: %w", err)
		}
		defer f.Close()
		bytesWritten, err = io.Copy(w, f)
		if err != nil {
			w.Close()
			return Response{}, fmt.Errorf("write GCS object: %w", err)
		}
	} else if req.Content != "" {
		bytesWritten = int64(len(req.Content))
		_, err = w.Write([]byte(req.Content))
		if err != nil {
			w.Close()
			return Response{}, fmt.Errorf("write GCS object: %w", err)
		}
	} else {
		w.Close()
		return Response{}, errors.New("either local_file or content must be provided")
	}

	if err := w.Close(); err != nil {
		return Response{}, fmt.Errorf("close GCS writer: %w", err)
	}

	attrs, err := obj.Attrs(ctx)
	if err != nil {
		return Response{
			URI:       fmt.Sprintf("gs://%s/%s", req.Bucket, req.Object),
			SizeBytes: bytesWritten,
		}, nil
	}

	return Response{
		URI:       fmt.Sprintf("gs://%s/%s", req.Bucket, req.Object),
		SizeBytes: attrs.Size,
		MD5:       fmt.Sprintf("%x", attrs.MD5),
		Updated:   attrs.Updated.Format(time.RFC3339),
	}, nil
}

func runGCSDownload(ctx context.Context, req Request) (Response, error) {
	if req.Bucket == "" || req.Object == "" || req.LocalFile == "" {
		return Response{}, errors.New("bucket, object and local_file are required")
	}

	client, err := storage.NewClient(ctx)
	if err != nil {
		return Response{}, fmt.Errorf("create storage client: %w", err)
	}
	defer client.Close()

	rc, err := client.Bucket(req.Bucket).Object(req.Object).NewReader(ctx)
	if err != nil {
		return Response{}, fmt.Errorf("read GCS object: %w", err)
	}
	defer rc.Close()

	if err := os.MkdirAll(filepath.Dir(req.LocalFile), 0o755); err != nil {
		return Response{}, fmt.Errorf("create destination directory: %w", err)
	}

	f, err := os.Create(req.LocalFile)
	if err != nil {
		return Response{}, fmt.Errorf("create local file: %w", err)
	}
	defer f.Close()

	written, err := io.Copy(f, rc)
	if err != nil {
		return Response{}, fmt.Errorf("download GCS file: %w", err)
	}

	return Response{
		LocalFile: req.LocalFile,
		SizeBytes: written,
	}, nil
}

// Pub/Sub handlers
func runPubSubPublish(ctx context.Context, req Request) (Response, error) {
	if req.ProjectID == "" || req.Topic == "" || req.Text == "" {
		return Response{}, errors.New("project_id, topic and text are required")
	}

	client, err := pubsub.NewClient(ctx, req.ProjectID)
	if err != nil {
		return Response{}, fmt.Errorf("create pubsub client: %w", err)
	}
	defer client.Close()

	topic := client.Topic(req.Topic)
	msg := &pubsub.Message{
		Data: []byte(req.Text),
	}
	if len(req.Attributes) > 0 {
		msg.Attributes = req.Attributes
	}

	res := topic.Publish(ctx, msg)
	msgID, err := res.Get(ctx)
	if err != nil {
		return Response{}, fmt.Errorf("publish message: %w", err)
	}

	return Response{
		MessageID: msgID,
	}, nil
}

func runPubSubPull(ctx context.Context, req Request) (Response, error) {
	if req.ProjectID == "" || req.Subscription == "" {
		return Response{}, errors.New("project_id and subscription are required")
	}

	client, err := pubsub.NewClient(ctx, req.ProjectID)
	if err != nil {
		return Response{}, fmt.Errorf("create pubsub client: %w", err)
	}
	defer client.Close()

	sub := client.Subscription(req.Subscription)
	max := req.MaxMessages
	if max <= 0 {
		max = 5
	}

	msgsChan := make(chan PubSubMessage, max)
	cctx, cancel := context.WithTimeout(ctx, 3*time.Second)
	defer cancel()

	var pulledCount int32
	go func() {
		_ = sub.Receive(cctx, func(ctx context.Context, m *pubsub.Message) {
			msgsChan <- PubSubMessage{
				MessageID:   m.ID,
				Data:        string(m.Data),
				Attributes:  m.Attributes,
				PublishTime: m.PublishTime.Format(time.RFC3339),
			}
			m.Ack()
			if atomic.AddInt32(&pulledCount, 1) >= int32(max) {
				cancel()
			}
		})
	}()

	<-cctx.Done()
	close(msgsChan)

	var msgs []PubSubMessage
	for m := range msgsChan {
		msgs = append(msgs, m)
	}

	return Response{
		Messages: msgs,
	}, nil
}

// BigQuery handler
func runBQQuery(ctx context.Context, req Request) (Response, error) {
	if req.ProjectID == "" || req.Prompt == "" {
		return Response{}, errors.New("project_id and prompt (SQL query) are required")
	}

	client, err := bigquery.NewClient(ctx, req.ProjectID)
	if err != nil {
		return Response{}, fmt.Errorf("create bigquery client: %w", err)
	}
	defer client.Close()

	q := client.Query(req.Prompt)
	it, err := q.Read(ctx)
	if err != nil {
		return Response{}, fmt.Errorf("read query results: %w", err)
	}

	schema := it.Schema
	var colNames []string
	for _, field := range schema {
		colNames = append(colNames, field.Name)
	}

	var results []map[string]bigquery.Value
	for {
		var row []bigquery.Value
		err := it.Next(&row)
		if errors.Is(err, iterator.Done) {
			break
		}
		if err != nil {
			return Response{}, fmt.Errorf("iterate row: %w", err)
		}

		mappedRow := make(map[string]bigquery.Value)
		for idx, val := range row {
			name := fmt.Sprintf("col_%d", idx)
			if idx < len(colNames) {
				name = colNames[idx]
			}
			mappedRow[name] = val
		}
		results = append(results, mappedRow)
	}

	data, err := json.Marshal(results)
	if err != nil {
		return Response{}, err
	}

	return Response{
		Text:      string(data),
		TotalRows: len(results),
	}, nil
}

// Secret Manager handler
func runSecretGet(ctx context.Context, req Request) (Response, error) {
	if req.ProjectID == "" || req.Name == "" {
		return Response{}, errors.New("project_id and name are required")
	}

	client, err := secretmanager.NewClient(ctx)
	if err != nil {
		return Response{}, fmt.Errorf("create secret manager client: %w", err)
	}
	defer client.Close()

	version := req.SecretVersion
	if version == "" {
		version = "latest"
	}

	secretPath := fmt.Sprintf("projects/%s/secrets/%s/versions/%s", req.ProjectID, req.Name, version)
	result, err := client.AccessSecretVersion(ctx, &secretmanagerpb.AccessSecretVersionRequest{
		Name: secretPath,
	})
	if err != nil {
		return Response{}, fmt.Errorf("access secret version: %w", err)
	}

	return Response{
		Text: string(result.Payload.Data),
	}, nil
}

// Firestore handler
func runFirestoreCRUD(ctx context.Context, req Request) (Response, error) {
	if req.ProjectID == "" || req.Name == "" {
		return Response{}, errors.New("project_id and name (collection/document path) are required")
	}

	client, err := firestore.NewClient(ctx, req.ProjectID)
	if err != nil {
		return Response{}, fmt.Errorf("create firestore client: %w", err)
	}
	defer client.Close()

	// Count segments to see if it is a collection or document path
	parts := strings.Split(strings.Trim(req.Name, "/"), "/")
	if len(parts)%2 == 1 {
		col := client.Collection(req.Name)
		iter := col.Documents(ctx)
		var docs []map[string]interface{}
		for {
			docSnap, err := iter.Next()
			if errors.Is(err, iterator.Done) {
				break
			}
			if err != nil {
				return Response{}, fmt.Errorf("iterate collection documents: %w", err)
			}
			data := docSnap.Data()
			data["_id"] = docSnap.Ref.ID
			docs = append(docs, data)
		}
		data, err := json.Marshal(docs)
		if err != nil {
			return Response{}, err
		}
		return Response{
			Text: string(data),
		}, nil
	}

	doc := client.Doc(req.Name)
	if req.Content != "" {
		var fields map[string]interface{}
		if err := json.Unmarshal([]byte(req.Content), &fields); err != nil {
			return Response{}, fmt.Errorf("parse content JSON: %w", err)
		}
		_, err = doc.Set(ctx, fields)
		if err != nil {
			return Response{}, fmt.Errorf("set firestore document: %w", err)
		}
		return Response{URI: fmt.Sprintf("firestore://%s/%s", req.ProjectID, req.Name)}, nil
	}

	snap, err := doc.Get(ctx)
	if err != nil {
		return Response{}, fmt.Errorf("get firestore document: %w", err)
	}

	data, err := json.Marshal(snap.Data())
	if err != nil {
		return Response{}, err
	}

	return Response{
		Text: string(data),
	}, nil
}

// KMS handler
func runKMSEncrypt(ctx context.Context, req Request) (Response, error) {
	if req.Name == "" || req.Text == "" {
		return Response{}, errors.New("name (key path) and text (plaintext) are required")
	}

	client, err := kms.NewKeyManagementClient(ctx)
	if err != nil {
		return Response{}, fmt.Errorf("create kms client: %w", err)
	}
	defer client.Close()

	result, err := client.Encrypt(ctx, &kmspb.EncryptRequest{
		Name:      req.Name,
		Plaintext: []byte(req.Text),
	})
	if err != nil {
		return Response{}, fmt.Errorf("kms encrypt: %w", err)
	}

	return Response{
		Text: string(result.Ciphertext),
	}, nil
}

func runKMSDecrypt(ctx context.Context, req Request) (Response, error) {
	if req.Name == "" || req.Text == "" {
		return Response{}, errors.New("name (key path) and text (ciphertext) are required")
	}

	client, err := kms.NewKeyManagementClient(ctx)
	if err != nil {
		return Response{}, fmt.Errorf("create kms client: %w", err)
	}
	defer client.Close()

	result, err := client.Decrypt(ctx, &kmspb.DecryptRequest{
		Name:       req.Name,
		Ciphertext: []byte(req.Text),
	})
	if err != nil {
		return Response{}, fmt.Errorf("kms decrypt: %w", err)
	}

	return Response{
		Text: string(result.Plaintext),
	}, nil
}

// Cloud Tasks handler
func runCloudTasksCreate(ctx context.Context, req Request) (Response, error) {
	if req.ProjectID == "" || req.Location == "" || req.Queue == "" || req.Text == "" {
		return Response{}, errors.New("project_id, location, queue and text (task url or payload) are required")
	}

	client, err := cloudtasks.NewClient(ctx)
	if err != nil {
		return Response{}, fmt.Errorf("create tasks client: %w", err)
	}
	defer client.Close()

	parent := fmt.Sprintf("projects/%s/locations/%s/queues/%s", req.ProjectID, req.Location, req.Queue)
	task := &cloudtaskspb.Task{
		MessageType: &cloudtaskspb.Task_HttpRequest{
			HttpRequest: &cloudtaskspb.HttpRequest{
				HttpMethod: cloudtaskspb.HttpMethod_POST,
				Url:        req.Text,
			},
		},
	}
	if req.Content != "" {
		task.GetHttpRequest().Body = []byte(req.Content)
	}

	createdTask, err := client.CreateTask(ctx, &cloudtaskspb.CreateTaskRequest{
		Parent: parent,
		Task:   task,
	})
	if err != nil {
		return Response{}, fmt.Errorf("create task: %w", err)
	}

	return Response{
		Name: createdTask.Name,
	}, nil
}

// Translate handler
func runTranslate(ctx context.Context, req Request) (Response, error) {
	if req.ProjectID == "" || req.Text == "" || req.Language == "" {
		return Response{}, errors.New("project_id, text and language (target) are required")
	}

	client, err := translate.NewTranslationClient(ctx)
	if err != nil {
		return Response{}, fmt.Errorf("create translation client: %w", err)
	}
	defer client.Close()

	parent := fmt.Sprintf("projects/%s/locations/global", req.ProjectID)
	resp, err := client.TranslateText(ctx, &translatepb.TranslateTextRequest{
		Parent:             parent,
		TargetLanguageCode: req.Language,
		Contents:           []string{req.Text},
	})
	if err != nil {
		return Response{}, fmt.Errorf("translate text: %w", err)
	}

	if len(resp.Translations) == 0 {
		return Response{}, errors.New("no translations returned")
	}

	return Response{
		Text: resp.Translations[0].TranslatedText,
	}, nil
}

// Speech-to-Text handler
func runSpeechToText(ctx context.Context, req Request) (Response, error) {
	if req.ProjectID == "" || req.Location == "" || req.FileURI == "" {
		return Response{}, errors.New("project_id, location and file_uri (GCS uri) are required")
	}

	client, err := speech.NewClient(ctx)
	if err != nil {
		return Response{}, fmt.Errorf("create speech client: %w", err)
	}
	defer client.Close()

	reqSpeech := &speechpb.RecognizeRequest{
		Recognizer: fmt.Sprintf("projects/%s/locations/%s/recognizers/_", req.ProjectID, req.Location),
		Config: &speechpb.RecognitionConfig{
			Features: &speechpb.RecognitionFeatures{
				EnableAutomaticPunctuation: true,
			},
		},
		AudioSource: &speechpb.RecognizeRequest_Uri{
			Uri: req.FileURI,
		},
	}

	resp, err := client.Recognize(ctx, reqSpeech)
	if err != nil {
		return Response{}, fmt.Errorf("recognize audio: %w", err)
	}

	var transcripts []string
	for _, res := range resp.Results {
		if len(res.Alternatives) > 0 {
			transcripts = append(transcripts, res.Alternatives[0].Transcript)
		}
	}

	var text string
	if len(transcripts) > 0 {
		text = transcripts[0]
	}

	return Response{
		Text: text,
	}, nil
}

// Vision handler
func runVisionAnnotate(ctx context.Context, req Request) (Response, error) {
	if req.FileURI == "" {
		return Response{}, errors.New("file_uri (GCS uri) is required")
	}

	client, err := vision.NewImageAnnotatorClient(ctx)
	if err != nil {
		return Response{}, fmt.Errorf("create vision client: %w", err)
	}
	defer client.Close()

	reqVision := &visionpb.BatchAnnotateImagesRequest{
		Requests: []*visionpb.AnnotateImageRequest{
			{
				Image: &visionpb.Image{
					Source: &visionpb.ImageSource{
						GcsImageUri: req.FileURI,
					},
				},
				Features: []*visionpb.Feature{
					{
						Type:       visionpb.Feature_LABEL_DETECTION,
						MaxResults: 10,
					},
				},
			},
		},
	}

	resp, err := client.BatchAnnotateImages(ctx, reqVision)
	if err != nil {
		return Response{}, fmt.Errorf("annotate image: %w", err)
	}

	if len(resp.Responses) == 0 {
		return Response{}, errors.New("no responses returned from vision API")
	}

	res := resp.Responses[0]
	if res.Error != nil {
		return Response{}, fmt.Errorf("vision API error: %s", res.Error.Message)
	}

	var annotations []string
	for _, label := range res.LabelAnnotations {
		annotations = append(annotations, fmt.Sprintf("%s (score: %.2f)", label.Description, label.Score))
	}

	data, err := json.Marshal(annotations)
	if err != nil {
		return Response{}, err
	}

	return Response{
		Text: string(data),
	}, nil
}

// Vision safe search annotator helper
func runVisionSafeSearch(ctx context.Context, req Request) (Response, error) {
	if req.FileURI == "" {
		return Response{}, errors.New("file_uri (GCS uri) is required")
	}

	client, err := vision.NewImageAnnotatorClient(ctx)
	if err != nil {
		return Response{}, fmt.Errorf("create vision client: %w", err)
	}
	defer client.Close()

	reqVision := &visionpb.BatchAnnotateImagesRequest{
		Requests: []*visionpb.AnnotateImageRequest{
			{
				Image: &visionpb.Image{
					Source: &visionpb.ImageSource{
						GcsImageUri: req.FileURI,
					},
				},
				Features: []*visionpb.Feature{
					{
						Type: visionpb.Feature_SAFE_SEARCH_DETECTION,
					},
				},
			},
		},
	}

	resp, err := client.BatchAnnotateImages(ctx, reqVision)
	if err != nil {
		return Response{}, fmt.Errorf("annotate image: %w", err)
	}

	if len(resp.Responses) == 0 {
		return Response{}, errors.New("no responses returned from vision API")
	}

	res := resp.Responses[0]
	if res.Error != nil {
		return Response{}, fmt.Errorf("vision API error: %s", res.Error.Message)
	}

	props := res.SafeSearchAnnotation
	if props == nil {
		return Response{}, errors.New("no safe search annotation returned")
	}

	likelihoods := map[string]string{
		"adult":    props.Adult.String(),
		"spoof":    props.Spoof.String(),
		"medical":  props.Medical.String(),
		"violence": props.Violence.String(),
		"racy":     props.Racy.String(),
	}

	data, err := json.Marshal(likelihoods)
	if err != nil {
		return Response{}, err
	}

	return Response{
		Text: string(data),
	}, nil
}

// Text-to-Speech handler
func runTextToSpeech(ctx context.Context, req Request) (Response, error) {
	if req.Text == "" && req.Content == "" {
		return Response{}, errors.New("text or content is required")
	}
	if req.OutputPath == "" {
		return Response{}, errors.New("output_path is required")
	}

	client, err := texttospeech.NewClient(ctx)
	if err != nil {
		return Response{}, fmt.Errorf("create texttospeech client: %w", err)
	}
	defer client.Close()

	text := req.Text
	if text == "" {
		text = req.Content
	}

	lang := req.Language
	if lang == "" {
		lang = "en-US"
	}

	voiceSelection := &texttospeechpb.VoiceSelectionParams{
		LanguageCode: lang,
	}
	if req.Voice != "" {
		voiceSelection.Name = req.Voice
	} else {
		voiceSelection.SsmlGender = texttospeechpb.SsmlVoiceGender_NEUTRAL
	}

	ttsReq := &texttospeechpb.SynthesizeSpeechRequest{
		Input: &texttospeechpb.SynthesisInput{
			InputSource: &texttospeechpb.SynthesisInput_Text{Text: text},
		},
		Voice: voiceSelection,
		AudioConfig: &texttospeechpb.AudioConfig{
			AudioEncoding: texttospeechpb.AudioEncoding_MP3,
		},
	}

	resp, err := client.SynthesizeSpeech(ctx, ttsReq)
	if err != nil {
		return Response{}, fmt.Errorf("synthesize speech: %w", err)
	}

	if err := os.MkdirAll(filepath.Dir(req.OutputPath), 0o755); err != nil {
		return Response{}, fmt.Errorf("create destination directory: %w", err)
	}

	if err := os.WriteFile(req.OutputPath, resp.AudioContent, 0o644); err != nil {
		return Response{}, fmt.Errorf("write audio file: %w", err)
	}

	return Response{
		LocalFile: req.OutputPath,
		SizeBytes: int64(len(resp.AudioContent)),
	}, nil
}

// Cloud Logging handlers
func runLoggingWrite(ctx context.Context, req Request) (Response, error) {
	if req.ProjectID == "" || req.LogName == "" || (req.Text == "" && req.Content == "") {
		return Response{}, errors.New("project_id, log_name, and text/content are required")
	}

	client, err := logging.NewClient(ctx, req.ProjectID)
	if err != nil {
		return Response{}, fmt.Errorf("create logging client: %w", err)
	}
	defer client.Close()

	logger := client.Logger(req.LogName)

	text := req.Text
	if text == "" {
		text = req.Content
	}

	severity := logging.Info
	if req.Severity != "" {
		switch strings.ToUpper(req.Severity) {
		case "DEBUG":
			severity = logging.Debug
		case "INFO":
			severity = logging.Info
		case "WARNING", "WARN":
			severity = logging.Warning
		case "ERROR":
			severity = logging.Error
		case "CRITICAL":
			severity = logging.Critical
		}
	}

	logger.Log(logging.Entry{
		Payload:  text,
		Severity: severity,
	})

	if err := logger.Flush(); err != nil {
		return Response{}, fmt.Errorf("flush logs: %w", err)
	}

	return Response{
		Text: "logged successfully",
	}, nil
}

func runLoggingList(ctx context.Context, req Request) (Response, error) {
	if req.ProjectID == "" || req.LogName == "" {
		return Response{}, errors.New("project_id and log_name are required")
	}

	client, err := logadmin.NewClient(ctx, req.ProjectID)
	if err != nil {
		return Response{}, fmt.Errorf("create logadmin client: %w", err)
	}
	defer client.Close()

	filter := fmt.Sprintf(`logName="projects/%s/logs/%s"`, req.ProjectID, req.LogName)
	if req.Filter != "" {
		filter = fmt.Sprintf(`logName="projects/%s/logs/%s" AND (%s)`, req.ProjectID, req.LogName, req.Filter)
	}

	it := client.Entries(ctx, logadmin.Filter(filter))
	var entries []string
	max := req.MaxMessages
	if max <= 0 {
		max = 10
	}

	for {
		entry, err := it.Next()
		if errors.Is(err, iterator.Done) {
			break
		}
		if err != nil {
			return Response{}, fmt.Errorf("fetch log entry: %w", err)
		}
		payloadStr := ""
		if strPayload, ok := entry.Payload.(string); ok {
			payloadStr = strPayload
		} else {
			bytes, err := json.Marshal(entry.Payload)
			if err == nil {
				payloadStr = string(bytes)
			} else {
				payloadStr = fmt.Sprintf("%v", entry.Payload)
			}
		}
		entries = append(entries, fmt.Sprintf("[%s] [%s] %s", entry.Timestamp.Format(time.RFC3339), entry.Severity, payloadStr))
		if len(entries) >= max {
			break
		}
	}

	return Response{
		Entries: entries,
	}, nil
}

// Cloud Run handler
func runCloudRunList(ctx context.Context, req Request) (Response, error) {
	if req.ProjectID == "" {
		return Response{}, errors.New("project_id is required")
	}

	client, err := run.NewServicesClient(ctx)
	if err != nil {
		return Response{}, fmt.Errorf("create services client: %w", err)
	}
	defer client.Close()

	location := req.Location
	if location == "" {
		location = "-"
	}

	parent := fmt.Sprintf("projects/%s/locations/%s", req.ProjectID, location)
	runReq := &runpb.ListServicesRequest{
		Parent: parent,
	}

	it := client.ListServices(ctx, runReq)
	var services []string
	for {
		service, err := it.Next()
		if errors.Is(err, iterator.Done) {
			break
		}
		if err != nil {
			return Response{}, fmt.Errorf("list services: %w", err)
		}
		services = append(services, fmt.Sprintf("%s (URI: %s)", service.Name, service.Uri))
	}

	return Response{
		Entries: services,
	}, nil
}

// Cloud Functions handler
func runCloudFunctionsList(ctx context.Context, req Request) (Response, error) {
	if req.ProjectID == "" {
		return Response{}, errors.New("project_id is required")
	}

	client, err := functions.NewFunctionClient(ctx)
	if err != nil {
		return Response{}, fmt.Errorf("create functions client: %w", err)
	}
	defer client.Close()

	location := req.Location
	if location == "" {
		location = "-"
	}

	parent := fmt.Sprintf("projects/%s/locations/%s", req.ProjectID, location)
	funcReq := &functionspb.ListFunctionsRequest{
		Parent: parent,
	}

	it := client.ListFunctions(ctx, funcReq)
	var funcs []string
	for {
		fn, err := it.Next()
		if errors.Is(err, iterator.Done) {
			break
		}
		if err != nil {
			return Response{}, fmt.Errorf("list functions: %w", err)
		}
		funcs = append(funcs, fmt.Sprintf("%s (State: %s, EntryPoint: %s)", fn.GetName(), fn.GetState().String(), fn.GetBuildConfig().GetEntryPoint()))
	}

	return Response{
		Entries: funcs,
	}, nil
}

// Spanner handlers
func runSpannerQuery(ctx context.Context, req Request) (Response, error) {
	if req.Name == "" || req.Prompt == "" {
		return Response{}, errors.New("name (spanner database path) and prompt (SQL query) are required")
	}

	client, err := spanner.NewClient(ctx, req.Name)
	if err != nil {
		return Response{}, fmt.Errorf("create spanner client: %w", err)
	}
	defer client.Close()

	stmt := spanner.NewStatement(req.Prompt)
	iter := client.Single().Query(ctx, stmt)
	defer iter.Stop()

	var results []map[string]interface{}
	for {
		row, err := iter.Next()
		if errors.Is(err, iterator.Done) {
			break
		}
		if err != nil {
			return Response{}, fmt.Errorf("fetch spanner row: %w", err)
		}

		names := row.ColumnNames()
		mappedRow := make(map[string]interface{})
		for _, name := range names {
			var val interface{}
			if err := row.ColumnByName(name, &val); err != nil {
				var raw spanner.GenericColumnValue
				if err := row.ColumnByName(name, &raw); err == nil {
					mappedRow[name] = fmt.Sprintf("%v", raw.Value)
				} else {
					mappedRow[name] = nil
				}
			} else {
				mappedRow[name] = val
			}
		}
		results = append(results, mappedRow)
	}

	data, err := json.Marshal(results)
	if err != nil {
		return Response{}, err
	}

	return Response{
		Text:      string(data),
		TotalRows: len(results),
	}, nil
}

func runSpannerWrite(ctx context.Context, req Request) (Response, error) {
	if req.Name == "" || req.Content == "" {
		return Response{}, errors.New("name (spanner database path) and content (SQL DML statement) are required")
	}

	client, err := spanner.NewClient(ctx, req.Name)
	if err != nil {
		return Response{}, fmt.Errorf("create spanner client: %w", err)
	}
	defer client.Close()

	_, err = client.ReadWriteTransaction(ctx, func(ctx context.Context, txn *spanner.ReadWriteTransaction) error {
		stmt := spanner.NewStatement(req.Content)
		_, err := txn.Update(ctx, stmt)
		return err
	})
	if err != nil {
		return Response{}, fmt.Errorf("spanner transaction: %w", err)
	}

	return Response{
		Text: "transaction completed successfully",
	}, nil
}
