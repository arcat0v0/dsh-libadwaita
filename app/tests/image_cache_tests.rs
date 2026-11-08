use dsh_libadwaita::ui::image_cache::{ImageCache, ImageClaim};

#[test]
fn test_claim_loads_once_then_serves_ready() {
    let mut cache: ImageCache<Vec<u8>> = ImageCache::default();
    assert_eq!(cache.claim("s1", "a1"), ImageClaim::Load);
    assert!(cache.is_pending("s1", "a1"));
    assert_eq!(cache.claim("s1", "a1"), ImageClaim::Pending);
    cache.complete("s1", "a1", vec![1, 2, 3]);
    assert_eq!(cache.claim("s1", "a1"), ImageClaim::Ready);
    assert_eq!(cache.peek("s1", "a1"), Some(&vec![1, 2, 3]));
    assert!(!cache.is_pending("s1", "a1"));
}

#[test]
fn test_cache_is_scoped_by_session() {
    let mut cache: ImageCache<Vec<u8>> = ImageCache::default();
    cache.complete("s1", "a1", vec![1]);
    assert_eq!(cache.claim("s2", "a1"), ImageClaim::Load);
    assert_eq!(cache.peek("s2", "a1"), None);
    assert_eq!(cache.peek("s1", "a1"), Some(&vec![1]));
}

#[test]
fn test_failed_load_can_retry() {
    let mut cache: ImageCache<Vec<u8>> = ImageCache::default();
    assert_eq!(cache.claim("s1", "a1"), ImageClaim::Load);
    cache.fail("s1", "a1");
    assert_eq!(cache.peek("s1", "a1"), None);
    assert_eq!(cache.claim("s1", "a1"), ImageClaim::Load);
    cache.complete("s1", "a1", vec![9]);
    assert_eq!(cache.claim("s1", "a1"), ImageClaim::Ready);
}

#[test]
fn test_seed_publishes_preview_without_reading() {
    let mut cache: ImageCache<Vec<u8>> = ImageCache::default();
    assert!(cache.seed("s1", "a1", vec![7]));
    assert_eq!(cache.claim("s1", "a1"), ImageClaim::Ready);
    assert!(!cache.seed("s1", "a1", vec![8]));
    assert_eq!(cache.peek("s1", "a1"), Some(&vec![7]));
}

#[test]
fn test_release_session_drops_only_that_scope() {
    let mut cache: ImageCache<Vec<u8>> = ImageCache::default();
    cache.complete("s1", "a1", vec![1]);
    cache.complete("s1", "a2", vec![2]);
    cache.complete("s2", "a1", vec![3]);
    assert_eq!(cache.release_session("s1"), 2);
    assert_eq!(cache.peek("s1", "a1"), None);
    assert_eq!(cache.peek("s2", "a1"), Some(&vec![3]));
    assert_eq!(cache.release_all(), 1);
    assert_eq!(cache.peek("s2", "a1"), None);
}
