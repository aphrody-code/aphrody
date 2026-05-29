# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors

"""Merge scraped data from JSON database into SQLite database."""

import os
import json
import sqlite3

DB_PATH = os.path.expanduser("~/.aphrody/x-store.sqlite")
JSON_PATH = "/home/ubuntu/.gemini/antigravity-cli/brain/915df5ef-84a3-4d37-a2c1-92f6e24b5e5c/scratch/beyblade_data.json"


def main():
    if not os.path.exists(JSON_PATH):
        print(f"JSON database not found at {JSON_PATH}!")
        return

    print("Loading JSON database...")
    with open(JSON_PATH, "r", encoding="utf-8") as f:
        data = json.load(f)

    tweets = data.get("tweets", {})
    users = data.get("users", {})

    print(f"Loaded {len(tweets)} tweets and {len(users)} users from JSON.")

    print(f"Connecting to SQLite database at {DB_PATH}...")
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    # Pre-load existing users from SQLite to avoid duplicates or look up user IDs
    cursor.execute("SELECT id, username, name FROM users")
    existing_users = cursor.fetchall()
    sqlite_user_ids = {row[0] for row in existing_users if row[0]}
    sqlite_user_usernames = {row[1].lower(): row[0] for row in existing_users if row[1]}

    # Build local user map from JSON data
    json_users_by_username = {}
    json_users_by_id = {}
    for key, u in users.items():
        uid = u.get("id")
        username = u.get("screen_name") or u.get("username") or ""
        if username:
            json_users_by_username[username.lower()] = u
        if uid:
            json_users_by_id[uid] = u

    # 1. Merge Users
    print("Merging users...")
    users_inserted = 0
    users_updated = 0

    for key, u in users.items():
        uid = u.get("id")
        username = u.get("screen_name") or u.get("username") or ""
        name = u.get("name") or username
        desc = u.get("description") or ""
        followers = u.get("followers_count") or 0
        following = u.get("friends_count") or u.get("following_count") or 0

        if not uid:
            # Try to resolve user ID from existing SQLite users or skip
            uid = sqlite_user_usernames.get(username.lower())
            if not uid:
                # If still not found, skip user insertion (will resolve from tweets if possible)
                continue

        user_json = json.dumps(u)

        if uid in sqlite_user_ids:
            # Update user stats
            cursor.execute(
                """UPDATE users SET 
                   username = ?, name = ?, description = ?, followers_count = ?, following_count = ?, json = ?
                   WHERE id = ?""",
                (username, name, desc, followers, following, user_json, uid)
            )
            users_updated += 1
        else:
            # Insert new user
            cursor.execute(
                """INSERT INTO users (id, username, name, description, followers_count, following_count, json)
                   VALUES (?, ?, ?, ?, ?, ?, ?)""",
                (uid, username, name, desc, followers, following, user_json)
            )
            sqlite_user_ids.add(uid)
            if username:
                sqlite_user_usernames[username.lower()] = uid
            users_inserted += 1

    print(f"Users: {users_inserted} inserted, {users_updated} updated.")

    # 2. Merge Tweets
    print("Merging tweets...")
    tweets_inserted = 0
    tweets_updated = 0
    fts_entries = 0

    for tid, t in tweets.items():
        text = t.get("text") or ""
        created_at = t.get("created_at")
        likes = t.get("like_count") or 0
        retweets = t.get("retweet_count") or 0
        replies = t.get("reply_count") or 0
        lang = t.get("lang") or ""
        author_username = t.get("author") or ""

        # Resolve author details
        author_name = ""
        author_id = None

        # Look up in our combined user map
        u_info = None
        if author_username and author_username.lower() in sqlite_user_usernames:
            author_id = sqlite_user_usernames[author_username.lower()]
        elif author_username and author_username.lower() in json_users_by_username:
            u_info = json_users_by_username[author_username.lower()]
            author_id = u_info.get("id")
            author_name = u_info.get("name") or ""

        # Check if tweet exists in SQLite
        cursor.execute("SELECT id FROM tweets WHERE id = ?", (tid,))
        exists = cursor.fetchone()

        tweet_json = json.dumps(t)

        if exists:
            # Update tweet engagement
            cursor.execute(
                """UPDATE tweets SET 
                   like_count = ?, retweet_count = ?, reply_count = ?, json = ?
                   WHERE id = ?""",
                (likes, retweets, replies, tweet_json, tid)
            )
            tweets_updated += 1
        else:
            # Insert new tweet
            cursor.execute(
                """INSERT INTO tweets 
                   (id, author_username, author_name, author_id, text, created_at, 
                    like_count, retweet_count, reply_count, quote_count, lang, json)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                (
                    tid,
                    author_username,
                    author_name,
                    author_id,
                    text,
                    created_at,
                    likes,
                    retweets,
                    replies,
                    0,  # quote_count default
                    lang,
                    tweet_json
                )
            )
            tweets_inserted += 1

            # Update FTS index for new tweets
            cursor.execute("DELETE FROM tweets_fts WHERE tweet_id = ?", (tid,))
            cursor.execute("INSERT INTO tweets_fts (text, tweet_id) VALUES (?, ?)", (text, tid))
            fts_entries += 1

    # Commit the changes
    conn.commit()
    conn.close()

    print(f"Tweets: {tweets_inserted} inserted, {tweets_updated} updated.")
    print(f"FTS entries updated: {fts_entries}")
    print("Database merge completed successfully!")


if __name__ == "__main__":
    main()
