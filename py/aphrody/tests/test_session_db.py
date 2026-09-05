from aphrody.session_db import SessionDB


def test_session_db_lifecycle(tmp_path):
    db_path = tmp_path / "test_sessions.db"
    db = SessionDB(db_path)
    assert db_path.exists()

    # Create session
    session_id = "test-session-123"
    db.create_session(session_id, source="pytest", model="gemini-flash")

    # Append message
    msg_id = db.append_message(
        session_id=session_id,
        role="user",
        content="Hello, this is a test message. We love python and testing.",
        token_count=10,
    )
    assert msg_id is not None
    assert msg_id > 0

    # Retrieve messages
    messages = db.get_messages(session_id)
    assert len(messages) == 1
    assert messages[0]["role"] == "user"
    assert (
        messages[0]["content"]
        == "Hello, this is a test message. We love python and testing."
    )
    assert messages[0]["token_count"] == 10

    db.close()


def test_session_db_fts_search(tmp_path):
    db_path = tmp_path / "test_search.db"
    db = SessionDB(db_path)

    session_id = "search-session"
    db.create_session(session_id, source="pytest")

    db.append_message(
        session_id, "user", "Deploying docker container to Kubernetes cluster"
    )
    db.append_message(
        session_id, "assistant", "Python is the best scripting language"
    )
    db.append_message(session_id, "user", "广西桂林漓江山水甲天下")  # CJK text

    # Search standard
    results = db.search_messages("docker container")
    assert len(results) == 1
    assert (
        "docker" in results[0]["snippet"].lower()
        or "kubernetes" in results[0]["snippet"].lower()
    )

    # Search CJK trigram (广西桂林漓江 -> >= 3 CJK chars)
    results_cjk = db.search_messages("广西桂林")
    assert len(results_cjk) == 1

    # Search CJK short LIKE fallback (广西 -> < 3 CJK chars)
    results_cjk_short = db.search_messages("广西")
    assert len(results_cjk_short) == 1

    db.close()


def test_query_sanitization():
    # Paired quotes
    assert SessionDB._sanitize_fts5_query('"exact phrase"') == '"exact phrase"'
    # Hyphenated/dotted terms wrapped in quotes
    assert SessionDB._sanitize_fts5_query("chat-send") == '"chat-send"'
    assert SessionDB._sanitize_fts5_query("P2.2") == '"P2.2"'
    # Special characters stripped
    assert SessionDB._sanitize_fts5_query("hello+world*") == "hello world*"
    # Boolean operators stripped from edges
    assert SessionDB._sanitize_fts5_query("AND hello OR") == "hello"
