use dsh_libadwaita::ui::controller::SessionController;

#[test]
fn test_reconnect_backoff_grows_and_caps() {
    assert_eq!(SessionController::next_reconnect_delay_ms(0), 1_000);
    assert_eq!(SessionController::next_reconnect_delay_ms(1), 2_000);
    assert_eq!(SessionController::next_reconnect_delay_ms(2), 4_000);
    assert_eq!(SessionController::next_reconnect_delay_ms(3), 8_000);
    assert_eq!(SessionController::next_reconnect_delay_ms(4), 16_000);
    assert_eq!(SessionController::next_reconnect_delay_ms(5), 30_000);
    assert_eq!(SessionController::next_reconnect_delay_ms(6), 30_000);
    assert_eq!(SessionController::next_reconnect_delay_ms(100), 30_000);
}
