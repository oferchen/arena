use leaderboard::LeaderboardService;

#[tokio::test]
async fn constructor_errors_without_redis_url() {
    std::env::remove_var("ARENA_REDIS_URL");
    let result = LeaderboardService::new("localhost:9042", std::env::temp_dir()).await;
    assert!(result.is_err());
}
