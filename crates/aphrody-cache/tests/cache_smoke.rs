// SPDX-License-Identifier: Apache-2.0
//! Executable contract for `aphrody-cache`.

use std::time::Duration;

use aphrody_cache::{
    Cache, CacheEntry, CacheKey, CachedBytes, DiskCache, LruCache, MemoryCache,
};
use chrono::Utc;

#[test]
fn cache_key_from_hash_is_hex_sha256() {
    // SHA-256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
    let k = CacheKey::from_hash(b"abc");
    assert_eq!(
        k.as_str(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(k.as_str().len(), 64);
}

#[test]
fn lru_evicts_oldest_on_capacity() {
    let mut lru: LruCache<u32> = LruCache::new(2).unwrap();
    let now = Utc::now();
    let a = CacheKey::from_string("a").unwrap();
    let b = CacheKey::from_string("b").unwrap();
    let c = CacheKey::from_string("c").unwrap();
    lru.put(a.clone(), 1, None, now);
    lru.put(b.clone(), 2, None, now);
    // Inserting c should evict a (oldest).
    lru.put(c.clone(), 3, None, now);
    assert_eq!(lru.len(), 2);
    assert!(lru.peek(&a).is_none(), "oldest should be evicted");
    assert!(lru.peek(&b).is_some());
    assert!(lru.peek(&c).is_some());
    assert!(lru.evictions() >= 1);
}

#[test]
fn lru_get_renews_recency() {
    let mut lru: LruCache<u32> = LruCache::new(2).unwrap();
    let now = Utc::now();
    let a = CacheKey::from_string("a").unwrap();
    let b = CacheKey::from_string("b").unwrap();
    let c = CacheKey::from_string("c").unwrap();
    lru.put(a.clone(), 1, None, now);
    lru.put(b.clone(), 2, None, now);
    // Touch a so it becomes most-recently-used.
    assert_eq!(lru.get(&a, now), Some(&1));
    // Now inserting c should evict b (now the LRU).
    lru.put(c.clone(), 3, None, now);
    assert!(lru.peek(&a).is_some(), "a was renewed and should survive");
    assert!(lru.peek(&b).is_none(), "b should be evicted as LRU");
    assert!(lru.peek(&c).is_some());
}

#[test]
fn entry_with_ttl_expires() {
    // Use a real-time sleep to validate the expiry path end-to-end.
    let created = Utc::now();
    let entry = CacheEntry::new(1u8, Some(50), created);
    std::thread::sleep(Duration::from_millis(100));
    let now = Utc::now();
    assert!(entry.is_expired(now), "entry with ttl=50ms must expire after 100ms");
}

#[test]
fn evict_expired_purges() {
    let mut lru: LruCache<u32> = LruCache::new(8).unwrap();
    let now = Utc::now();
    lru.put(CacheKey::from_string("a").unwrap(), 1, Some(10), now);
    lru.put(CacheKey::from_string("b").unwrap(), 2, Some(10), now);
    lru.put(CacheKey::from_string("c").unwrap(), 3, None, now);
    std::thread::sleep(Duration::from_millis(50));
    let later = Utc::now();
    let n = lru.evict_expired(later);
    assert_eq!(n, 2, "two ttl entries must be evicted");
    assert_eq!(lru.len(), 1);
    assert!(lru.peek(&CacheKey::from_string("c").unwrap()).is_some());
}

#[tokio::test]
async fn memory_cache_get_miss_returns_none() {
    let cache = MemoryCache::new(8);
    let key = CacheKey::from_hash(b"never-stored");
    let got = cache.get(&key).await.unwrap();
    assert!(got.is_none());
    let stats = cache.stats().await;
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.hits, 0);
}

#[tokio::test]
async fn memory_cache_put_then_get_returns_value() {
    let cache = MemoryCache::new(8);
    let key = CacheKey::from_hash(b"hello-world");
    let payload = CachedBytes::from(b"the-payload".to_vec());
    cache.put(&key, payload.clone(), None).await.unwrap();
    let got = cache.get(&key).await.unwrap().expect("hit");
    assert_eq!(got, payload);
    let stats = cache.stats().await;
    assert_eq!(stats.puts, 1);
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.entries, 1);
}

#[tokio::test]
async fn disk_cache_persists_across_instances() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().to_path_buf();
    let key = CacheKey::from_hash(b"persistent");
    let payload = CachedBytes::from(b"durable-bytes".to_vec());

    {
        let cache = DiskCache::open(&dir).await.expect("open");
        cache.put(&key, payload.clone(), None).await.expect("put");
    }
    // Drop the first instance, reopen against the same dir.
    {
        let cache = DiskCache::open(&dir).await.expect("reopen");
        let got = cache.get(&key).await.expect("get").expect("hit");
        assert_eq!(got, payload);
        let stats = cache.stats().await;
        assert_eq!(stats.entries, 1);
        assert_eq!(stats.hits, 1);
    }
}

#[tokio::test]
async fn disk_cache_respects_ttl() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let cache = DiskCache::open(tmp.path()).await.unwrap();
    let key = CacheKey::from_hash(b"ttl-victim");
    cache
        .put(&key, CachedBytes::from(b"x".to_vec()), Some(10))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(80)).await;
    let got = cache.get(&key).await.unwrap();
    assert!(got.is_none(), "ttl=10ms entry must be evicted after 80ms");
}

#[tokio::test]
async fn cache_stats_track_hits_and_misses() {
    let cache = MemoryCache::new(4);
    let key_hit = CacheKey::from_hash(b"hit");
    let key_miss = CacheKey::from_hash(b"miss");
    cache
        .put(&key_hit, CachedBytes::from(b"v".to_vec()), None)
        .await
        .unwrap();
    // 2 hits + 3 misses.
    assert!(cache.get(&key_hit).await.unwrap().is_some());
    assert!(cache.get(&key_hit).await.unwrap().is_some());
    assert!(cache.get(&key_miss).await.unwrap().is_none());
    assert!(cache.get(&key_miss).await.unwrap().is_none());
    assert!(cache.get(&key_miss).await.unwrap().is_none());
    let stats = cache.stats().await;
    assert_eq!(stats.hits, 2);
    assert_eq!(stats.misses, 3);
    assert_eq!(stats.puts, 1);
}

#[tokio::test]
async fn memory_cache_delete_and_clear() {
    let cache = MemoryCache::new(4);
    let key = CacheKey::from_hash(b"to-delete");
    cache
        .put(&key, CachedBytes::from(b"v".to_vec()), None)
        .await
        .unwrap();
    cache.delete(&key).await.unwrap();
    assert!(cache.get(&key).await.unwrap().is_none());
    // Re-populate and clear.
    cache
        .put(&key, CachedBytes::from(b"v".to_vec()), None)
        .await
        .unwrap();
    cache.clear().await.unwrap();
    let stats = cache.stats().await;
    assert_eq!(stats.entries, 0);
}
