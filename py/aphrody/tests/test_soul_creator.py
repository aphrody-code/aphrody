# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Unit tests for the SoulCreator media crawler and parser."""

from unittest.mock import MagicMock, patch

from aphrody.soul_creator import MediaHTMLParser, SoulCreator


def test_media_parser_wikipedia_format():
    parser = MediaHTMLParser()
    html = """
    <h1 id="firstHeading">The Matrix</h1>
    <table class="infobox">
        <tr>
            <th>Directed by</th>
            <td>The Wachowskis</td>
        </tr>
        <tr>
            <th>Released</th>
            <td>March 31, 1999</td>
        </tr>
    </table>
    <h2>Plot Summary</h2>
    <p>A computer hacker learns from mysterious rebels about the true nature of his reality...</p>
    <li><a href="/wiki/Neo_(The_Matrix)">Neo</a></li>
    """
    parser.feed(html)

    assert parser.title == "The Matrix"
    assert parser.infobox.get("Directed by") == "The Wachowskis"
    assert parser.infobox.get("Released") == "March 31, 1999"
    assert ("h2", "Plot Summary") in parser.headings
    assert "A computer hacker learns" in "".join(parser.text_content)
    assert "/wiki/Neo_(The_Matrix)" in parser.wiki_links


def test_media_parser_fandom_format():
    parser = MediaHTMLParser()
    html = """
    <div class="page-header__title">Geralt of Rivia</div>
    <div class="portable-infobox">
        <div class="pi-item">
            <h3 class="pi-data-label">Race</h3>
            <div class="pi-data-value">Witcher</div>
        </div>
        <div class="pi-item">
            <h3 class="pi-data-label">Affiliation</h3>
            <div class="pi-data-value">School of the Wolf</div>
        </div>
    </div>
    <p>Geralt is a legendary monster hunter...</p>
    <a href="/wiki/Yennefer">Yennefer of Vengerberg</a>
    """
    parser.feed(html)

    assert parser.title == "Geralt of Rivia"
    assert parser.infobox.get("Race") == "Witcher"
    assert parser.infobox.get("Affiliation") == "School of the Wolf"
    assert "Geralt is a legendary" in "".join(parser.text_content)
    assert "/wiki/Yennefer" in parser.wiki_links


@patch("httpx.Client")
def test_soul_creator_scrape_url(mock_client_class):
    # Setup mock httpx client response
    mock_client = MagicMock()
    mock_response = MagicMock()
    mock_response.text = """
    <h1 id="firstHeading">Zelda</h1>
    <table class="infobox">
        <tr>
            <th>Developer</th>
            <td>Nintendo</td>
        </tr>
    </table>
    <p>The Legend of Zelda is a high-fantasy action-adventure game franchise...</p>
    """
    mock_client.get.return_value = mock_response
    mock_client_class.return_value.__enter__.return_value = mock_client

    creator = SoulCreator()
    res = creator.scrape_url(
        "https://en.wikipedia.org/wiki/The_Legend_of_Zelda"
    )

    assert res["success"] is True
    assert res["title"] == "Zelda"
    assert res["infobox"].get("Developer") == "Nintendo"
    assert "The Legend of Zelda is" in res["body"]


def test_soul_creator_format_markdown():
    creator = SoulCreator()
    data = {
        "success": True,
        "url": "https://en.wikipedia.org/wiki/Mario",
        "title": "Mario",
        "infobox": {
            "First appearance": "Donkey Kong (1981)",
            "Creator": "Shigeru Miyamoto",
        },
        "body": "Mario is a fictional character in the Mario video game franchise...",
        "links": ["https://en.wikipedia.org/wiki/Luigi"],
    }

    markdown = creator.format_profile_markdown(data)

    assert "# Profile: Mario" in markdown
    assert "**Source URL**: https://en.wikipedia.org/wiki/Mario" in markdown
    assert "**Creator**: Shigeru Miyamoto" in markdown
    assert "Mario is a fictional character" in markdown
    assert "- [Luigi](https://en.wikipedia.org/wiki/Luigi)" in markdown


def test_soul_creator_create_agent_soul():
    creator = SoulCreator()
    profile = {
        "title": "Aphrody (Byron Love)",
        "infobox": {
            "Position": "MF",
            "Element": "Forest",
        },
        "body": "Captivating the opposition with artistic grace, he plays with the aura of a deity from on high.",
    }

    prompt = creator.create_agent_soul(profile)

    assert "# Agent Soul & Personality: Aphrody (Byron Love)" in prompt
    assert (
        "Tone & Style**: supremely confident, elegant, and charismatic (inspired by Aphrody)"
        in prompt
    )
    assert "Heaven's Time" in prompt
    assert "God Knows" in prompt
    assert "Chaos Break" in prompt

    # Test French version
    prompt_fr = creator.create_agent_soul(profile, lang="fr")
    assert "# Âme d'Agent & Personnalité : Aphrody (Byron Love)" in prompt_fr
    assert (
        "Ton & Style** : extrêmement confiant, élégant et charismatique (inspiré d'Aphrody / Byron Love)"
        in prompt_fr
    )
    assert "Instant Céleste" in prompt_fr
    assert "Savoir Suprême" in prompt_fr
    assert "Tir Chaotique" in prompt_fr

    # Test generic character
    generic_profile = {
        "title": "Sherlock Holmes",
        "infobox": {
            "Special Ability": "Deduction",
        },
        "body": "A genius consulting detective known for his analytical and highly intelligent nature.",
    }

    generic_prompt = creator.create_agent_soul(generic_profile)
    assert "# Agent Soul & Personality: Sherlock Holmes" in generic_prompt
    assert (
        "analytical, precise, and highly intellectual (inspired by Sherlock Holmes)"
        in generic_prompt
    )
    assert "Deduction" in generic_prompt

    # Test generic French character
    generic_prompt_fr = creator.create_agent_soul(generic_profile, lang="fr")
    assert "# Âme d'Agent & Personnalité : Sherlock Holmes" in generic_prompt_fr
    assert (
        "analytique, précis et hautement intellectuel (inspiré par Sherlock Holmes)"
        in generic_prompt_fr
    )
