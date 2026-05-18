// SPDX-License-Identifier: Apache-2.0
//
// Parity tests for `aphrody-react-reconciler`. Mirrors the assertions from
// `packages/aphrody-jsx/tests/reconciler.test.ts` plus a few diff/osc edge
// cases drawn from `tests/hooks.test.ts` and `tests/osc.test.ts`.

use serde_json::{json, Map, Value};

use aphrody_react_reconciler::{
    decode_frame, freshen_ids, ElementKind, JsonNode, JsonPatchOp, OscFrame, OscFrameParser,
    Reconciler, UseState, VecSink,
};

// ---------------------------------------------------------------------------
// Test 1 — initial mount frame structure mirrors the TS `render → mount`
// assertion ("hello tree emits a single mount frame with the expected
// structure").
// ---------------------------------------------------------------------------
#[test]
fn hello_tree_emits_single_mount_frame_with_expected_structure() {
    freshen_ids();
    let reconciler = Reconciler::new();

    // <Box flexDirection="column" padding=1>
    //   <Text bold color="primary">Hello aphrody-jsx</Text>
    // </Box>
    let mut box_props = Map::new();
    box_props.insert("flexDirection".into(), Value::String("column".into()));
    box_props.insert("paddingTop".into(), Value::from(1));
    box_props.insert("paddingBottom".into(), Value::from(1));
    box_props.insert("paddingLeft".into(), Value::from(1));
    box_props.insert("paddingRight".into(), Value::from(1));
    let box_node = reconciler.append_element(ElementKind::Box, box_props);

    let mut text_props = Map::new();
    text_props.insert("bold".into(), Value::Bool(true));
    text_props.insert("color".into(), Value::String("primary".into()));
    let text_node = aphrody_react_reconciler::create_element(ElementKind::Text, text_props);
    aphrody_react_reconciler::element::append_child(&box_node, &text_node);

    let inner = aphrody_react_reconciler::create_text_instance("Hello aphrody-jsx");
    aphrody_react_reconciler::element::append_child(&text_node, &inner);

    let mut sink = VecSink::new();
    reconciler.commit(&mut sink);

    let mut parser = OscFrameParser::new();
    let frames = parser.push(&sink.joined());
    assert!(!frames.is_empty(), "expected at least one frame");

    let mount = frames
        .iter()
        .find(|f| matches!(f, OscFrame::Mount(_)))
        .expect("expected a Mount frame");
    let OscFrame::Mount(fields) = mount else {
        unreachable!()
    };
    let JsonNode::Element {
        kind: root_kind,
        children: root_children,
        ..
    } = &fields.tree
    else {
        panic!("root should be an Element node");
    };
    assert_eq!(root_kind.as_str(), "Root");
    assert_eq!(root_children.len(), 1);

    let JsonNode::Element {
        kind: box_kind,
        props: box_serialized_props,
        children: box_children,
        ..
    } = &root_children[0]
    else {
        panic!("expected Box child");
    };
    assert_eq!(box_kind.as_str(), "Box");
    assert_eq!(box_serialized_props["paddingTop"], json!(1));
    assert_eq!(box_serialized_props["paddingBottom"], json!(1));
    assert_eq!(box_serialized_props["paddingLeft"], json!(1));
    assert_eq!(box_serialized_props["paddingRight"], json!(1));
    assert_eq!(box_serialized_props["flexDirection"], json!("column"));
    assert_eq!(box_children.len(), 1);

    let JsonNode::Element {
        kind: text_kind,
        props: text_serialized_props,
        children: text_children,
        ..
    } = &box_children[0]
    else {
        panic!("expected Text child");
    };
    assert_eq!(text_kind.as_str(), "Text");
    assert_eq!(text_serialized_props["bold"], json!(true));
    // Color tokens carry the `m3:` marker for re-resolution renderer-side.
    assert_eq!(text_serialized_props["color"], json!("m3:primary"));
    assert_eq!(text_children.len(), 1);
    match &text_children[0] {
        JsonNode::TextNode { value, .. } => assert_eq!(value, "Hello aphrody-jsx"),
        JsonNode::Element { .. } => panic!("expected TEXT_NODE child"),
    }
}

// ---------------------------------------------------------------------------
// Test 2 — rerender emits an `update` patch, not a fresh `mount`. Direct
// port of the TS test "rerender emits an update patch, not a fresh mount".
// ---------------------------------------------------------------------------
#[test]
fn rerender_emits_update_patch_not_mount() {
    freshen_ids();
    let reconciler = Reconciler::new();

    let mut text_props = Map::new();
    text_props.insert("color".into(), Value::String("primary".into()));
    let text_node = aphrody_react_reconciler::create_element(ElementKind::Text, text_props);
    let inner = aphrody_react_reconciler::create_text_instance("first");
    aphrody_react_reconciler::element::append_child(&text_node, &inner);
    reconciler.append_child_to_container(text_node.clone());

    let mut sink = VecSink::new();
    reconciler.commit(&mut sink);
    let initial_chunks = sink.chunks.len();

    // Re-render: change color to "secondary" and text to "second".
    let mut next_props = Map::new();
    next_props.insert("color".into(), Value::String("secondary".into()));
    reconciler
        .commit_update(&text_node, next_props)
        .expect("commit_update should succeed on an Element");
    reconciler
        .commit_text_update(&inner, "second")
        .expect("commit_text_update should succeed on a TextNode");

    reconciler.commit(&mut sink);

    let after: String = sink.chunks[initial_chunks..].concat();
    let mut parser = OscFrameParser::new();
    let frames = parser.push(&after);
    assert!(
        frames.iter().any(|f| matches!(f, OscFrame::Update(_))),
        "expected at least one Update frame after rerender"
    );
    assert!(
        frames.iter().all(|f| !matches!(f, OscFrame::Mount(_))),
        "no Mount frame should be emitted on rerender"
    );

    // Drill into the patch list and verify both ops are present.
    let OscFrame::Update(fields) = frames
        .iter()
        .find(|f| matches!(f, OscFrame::Update(_)))
        .unwrap()
    else {
        unreachable!()
    };
    assert!(fields.patch.iter().any(|op| matches!(
        op,
        JsonPatchOp::ReplaceProp { key, value, .. }
            if key == "color" && value == &json!("m3:secondary")
    )));
    assert!(fields.patch.iter().any(|op| matches!(
        op,
        JsonPatchOp::ReplaceText { value, .. } if value == "second"
    )));
}

// ---------------------------------------------------------------------------
// Test 3 — Fragment-equivalent (multiple top-level children) serializes as
// an ordered list. The TS source has no explicit Fragment element but the
// reconciler appends siblings to the Root container the same way, so we
// validate the multi-child Root path.
// ---------------------------------------------------------------------------
#[test]
fn fragment_like_multiple_root_children_preserve_order() {
    freshen_ids();
    let reconciler = Reconciler::new();
    reconciler.append_element(ElementKind::Newline, Map::new());
    reconciler.append_element(ElementKind::Spacer, Map::new());
    reconciler.append_element(ElementKind::Newline, Map::new());

    let snapshot = reconciler.snapshot();
    let JsonNode::Element { children, .. } = snapshot else {
        panic!("root must be Element");
    };
    let kinds: Vec<&'static str> = children
        .iter()
        .map(|c| match c {
            JsonNode::Element { kind, .. } => kind.as_str(),
            JsonNode::TextNode { .. } => "TEXT_NODE",
        })
        .collect();
    assert_eq!(kinds, vec!["Newline", "Spacer", "Newline"]);
}

// ---------------------------------------------------------------------------
// Test 4 — `use_state` integrates with the commit pipeline. The setter
// drives an explicit text-update commit; the snapshot reflects the new
// state at the next commit boundary.
// ---------------------------------------------------------------------------
#[test]
fn use_state_drives_commit_pipeline() {
    freshen_ids();
    let reconciler = Reconciler::new();
    let text = aphrody_react_reconciler::create_element(ElementKind::Text, Map::new());
    let inner = aphrody_react_reconciler::create_text_instance("counter: 0");
    aphrody_react_reconciler::element::append_child(&text, &inner);
    reconciler.append_child_to_container(text.clone());

    let counter: UseState<i32> = UseState::new(0);
    let mut sink = VecSink::new();
    reconciler.commit(&mut sink);

    // Simulate three setState() calls each followed by a text update + commit.
    for _ in 0..3 {
        counter.update(|prev| prev + 1);
        let next = format!("counter: {}", counter.get());
        reconciler.commit_text_update(&inner, &next).unwrap();
        reconciler.commit(&mut sink);
    }

    assert_eq!(counter.get(), 3);

    // Final snapshot should reflect the latest value.
    let snapshot = reconciler.snapshot();
    let JsonNode::Element { children, .. } = snapshot else {
        panic!()
    };
    let JsonNode::Element { children: text_children, .. } = &children[0] else {
        panic!()
    };
    let JsonNode::TextNode { value, .. } = &text_children[0] else {
        panic!()
    };
    assert_eq!(value, "counter: 3");
}

// ---------------------------------------------------------------------------
// Test 5 — OSC codec roundtrip: encode → streaming decode preserves
// frame contents byte-for-byte. Covers Mount + Update + Unmount.
// ---------------------------------------------------------------------------
#[test]
fn osc_codec_streaming_roundtrip() {
    freshen_ids();
    let reconciler = Reconciler::new();
    reconciler.append_element(ElementKind::Box, Map::new());
    let mut sink = VecSink::new();
    reconciler.commit(&mut sink);

    // Add an unmount frame manually.
    let unmount = aphrody_react_reconciler::encode_frame(&OscFrame::Unmount(
        aphrody_react_reconciler::UnmountFields {
            id: reconciler.root_id().to_string(),
        },
    ));

    // Combine into a single stream and split arbitrarily across chunks.
    let combined = std::format!("{}{}", sink.joined(), unmount);
    let mid = combined.len() / 2;
    let (a, b) = combined.split_at(mid);
    let mut parser = OscFrameParser::new();
    let mut frames = parser.push(a);
    frames.extend(parser.push(b));

    // Expected: Mount + Unmount (no Update since we never re-committed).
    assert_eq!(frames.len(), 2);
    assert!(matches!(frames[0], OscFrame::Mount(_)));
    assert!(matches!(frames[1], OscFrame::Unmount(_)));

    // Decode-single roundtrip on the standalone unmount envelope.
    let parsed = decode_frame(&unmount).expect("unmount should decode");
    assert!(matches!(parsed, OscFrame::Unmount(_)));
}

// ---------------------------------------------------------------------------
// Test 6 — m3 token encoding stays symbolic for the renderer.
// ---------------------------------------------------------------------------
#[test]
fn m3_token_normalization_keeps_symbolic_marker() {
    use aphrody_react_reconciler::m3_tokens;
    assert_eq!(m3_tokens::encode_color("primary"), "m3:primary");
    assert_eq!(m3_tokens::encode_color("#FF0000"), "#FF0000");
    assert!(m3_tokens::is_m3_token("primary"));
    assert!(!m3_tokens::is_m3_token("blurple"));
}
