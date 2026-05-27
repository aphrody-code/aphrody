from aphrody.background_review import BackgroundReview


def test_background_review_parsing():
    reviewer = BackgroundReview()

    # Standard JSON
    res = reviewer._parse_json_response(
        '{"memory_update": "hello", "skills_update": null}'
    )
    assert res is not None
    assert res["memory_update"] == "hello"

    # JSON inside code block
    res_code = reviewer._parse_json_response(
        '```json\n{"memory_update": "hello", "skills_update": null}\n```'
    )
    assert res_code is not None
    assert res_code["memory_update"] == "hello"

    # Malformed leading/trailing junk
    res_junk = reviewer._parse_json_response(
        'Some text here: {"memory_update": "hello", "skills_update": null} trailing junk'
    )
    assert res_junk is not None
    assert res_junk["memory_update"] == "hello"


def test_background_review_apply_updates(tmp_path):
    mem_file = tmp_path / "memory.md"
    skills_dir = tmp_path / "skills"

    reviewer = BackgroundReview(
        memory_file=mem_file,
        skills_dir=skills_dir,
    )

    updates = {
        "memory_update": "User likes Python.",
        "skills_update": {
            "skill_name": "build",
            "file_path": "build.md",
            "content": "Run uv run pytest to build.",
        },
    }

    reviewer._apply_updates(updates)

    # Verify memory update written
    assert mem_file.exists()
    mem_content = mem_file.read_text(encoding="utf-8")
    assert "User likes Python." in mem_content

    # Verify skills update written
    skill_file = skills_dir / "build.md"
    assert skill_file.exists()
    skill_content = skill_file.read_text(encoding="utf-8")
    assert "Run uv run pytest to build." in skill_content
    assert "# Skill: build" in skill_content
