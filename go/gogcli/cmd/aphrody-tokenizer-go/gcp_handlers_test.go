package main

import (
	"context"
	"testing"
)

func TestGCSUpload_Validation(t *testing.T) {
	ctx := context.Background()

	// Missing bucket and object should return error
	req := Request{
		Command: "gcs_upload",
	}
	_, err := runGCSUpload(ctx, req)
	if err == nil {
		t.Error("expected error for missing bucket and object, got nil")
	}

	// Missing content and local file should return error
	req = Request{
		Command: "gcs_upload",
		Bucket:  "my-bucket",
		Object:  "my-object",
	}
	_, err = runGCSUpload(ctx, req)
	if err == nil {
		t.Error("expected error for missing local_file and content, got nil")
	}
}

func TestGCSDownload_Validation(t *testing.T) {
	ctx := context.Background()

	// Missing fields should return error
	req := Request{
		Command: "gcs_download",
	}
	_, err := runGCSDownload(ctx, req)
	if err == nil {
		t.Error("expected error for missing fields, got nil")
	}
}

func TestPubSubPublish_Validation(t *testing.T) {
	ctx := context.Background()

	req := Request{
		Command: "pubsub_publish",
	}
	_, err := runPubSubPublish(ctx, req)
	if err == nil {
		t.Error("expected error for missing fields, got nil")
	}
}

func TestPubSubPull_Validation(t *testing.T) {
	ctx := context.Background()

	req := Request{
		Command: "pubsub_pull",
	}
	_, err := runPubSubPull(ctx, req)
	if err == nil {
		t.Error("expected error for missing fields, got nil")
	}
}

func TestBQQuery_Validation(t *testing.T) {
	ctx := context.Background()

	req := Request{
		Command: "bq_query",
	}
	_, err := runBQQuery(ctx, req)
	if err == nil {
		t.Error("expected error for missing fields, got nil")
	}
}

func TestSecretGet_Validation(t *testing.T) {
	ctx := context.Background()

	req := Request{
		Command: "secret_get",
	}
	_, err := runSecretGet(ctx, req)
	if err == nil {
		t.Error("expected error for missing fields, got nil")
	}
}

func TestFirestoreCRUD_Validation(t *testing.T) {
	ctx := context.Background()

	req := Request{
		Command: "firestore_crud",
	}
	_, err := runFirestoreCRUD(ctx, req)
	if err == nil {
		t.Error("expected error for missing fields, got nil")
	}
}

func TestKMSEncrypt_Validation(t *testing.T) {
	ctx := context.Background()

	req := Request{
		Command: "kms_encrypt",
	}
	_, err := runKMSEncrypt(ctx, req)
	if err == nil {
		t.Error("expected error for missing fields, got nil")
	}
}

func TestKMSDecrypt_Validation(t *testing.T) {
	ctx := context.Background()

	req := Request{
		Command: "kms_decrypt",
	}
	_, err := runKMSDecrypt(ctx, req)
	if err == nil {
		t.Error("expected error for missing fields, got nil")
	}
}

func TestCloudTasksCreate_Validation(t *testing.T) {
	ctx := context.Background()

	req := Request{
		Command: "tasks_create",
	}
	_, err := runCloudTasksCreate(ctx, req)
	if err == nil {
		t.Error("expected error for missing fields, got nil")
	}
}

func TestTranslate_Validation(t *testing.T) {
	ctx := context.Background()

	req := Request{
		Command: "translate",
	}
	_, err := runTranslate(ctx, req)
	if err == nil {
		t.Error("expected error for missing fields, got nil")
	}
}

func TestSpeechToText_Validation(t *testing.T) {
	ctx := context.Background()

	req := Request{
		Command: "speech_to_text",
	}
	_, err := runSpeechToText(ctx, req)
	if err == nil {
		t.Error("expected error for missing fields, got nil")
	}
}

func TestVisionAnnotate_Validation(t *testing.T) {
	ctx := context.Background()

	req := Request{
		Command: "vision_annotate",
	}
	_, err := runVisionAnnotate(ctx, req)
	if err == nil {
		t.Error("expected error for missing fields, got nil")
	}
}

func TestVisionSafeSearch_Validation(t *testing.T) {
	ctx := context.Background()

	req := Request{
		Command: "vision_safesearch",
	}
	_, err := runVisionSafeSearch(ctx, req)
	if err == nil {
		t.Error("expected error for missing fields, got nil")
	}
}

func TestTextToSpeech_Validation(t *testing.T) {
	ctx := context.Background()

	req := Request{
		Command: "tts_synthesize",
	}
	_, err := runTextToSpeech(ctx, req)
	if err == nil {
		t.Error("expected error for missing fields, got nil")
	}
}

func TestLoggingWrite_Validation(t *testing.T) {
	ctx := context.Background()

	req := Request{
		Command: "logging_write",
	}
	_, err := runLoggingWrite(ctx, req)
	if err == nil {
		t.Error("expected error for missing fields, got nil")
	}
}

func TestLoggingList_Validation(t *testing.T) {
	ctx := context.Background()

	req := Request{
		Command: "logging_list",
	}
	_, err := runLoggingList(ctx, req)
	if err == nil {
		t.Error("expected error for missing fields, got nil")
	}
}

func TestCloudRunList_Validation(t *testing.T) {
	ctx := context.Background()

	req := Request{
		Command: "cloudrun_list",
	}
	_, err := runCloudRunList(ctx, req)
	if err == nil {
		t.Error("expected error for missing fields, got nil")
	}
}

func TestCloudFunctionsList_Validation(t *testing.T) {
	ctx := context.Background()

	req := Request{
		Command: "cloudfunctions_list",
	}
	_, err := runCloudFunctionsList(ctx, req)
	if err == nil {
		t.Error("expected error for missing fields, got nil")
	}
}

func TestSpannerQuery_Validation(t *testing.T) {
	ctx := context.Background()

	req := Request{
		Command: "spanner_query",
	}
	_, err := runSpannerQuery(ctx, req)
	if err == nil {
		t.Error("expected error for missing fields, got nil")
	}
}

func TestSpannerWrite_Validation(t *testing.T) {
	ctx := context.Background()

	req := Request{
		Command: "spanner_write",
	}
	_, err := runSpannerWrite(ctx, req)
	if err == nil {
		t.Error("expected error for missing fields, got nil")
	}
}

func TestGmailClean_Validation_Handler(t *testing.T) {
	ctx := context.Background()
	t.Setenv("GOG_AUTH_MODE", "")
	t.Setenv("GOG_ACCOUNT", "")

	req := Request{
		Command:      "gmail_clean",
		AccountEmail: "",
	}
	_, err := runGmailClean(ctx, req)
	if err == nil {
		t.Error("expected error when account email is empty and ADC is off, got nil")
	}
}
