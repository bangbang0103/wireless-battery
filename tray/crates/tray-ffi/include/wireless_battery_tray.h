#ifndef WIRELESS_BATTERY_TRAY_H
#define WIRELESS_BATTERY_TRAY_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Opaque handle owning the background tray service and polling thread. */
typedef struct WbTrayHandle WbTrayHandle;

/**
 * Callback invoked from the background poller thread whenever a new snapshot
 * or low-battery alert payload is available.
 *
 * `update_json_utf8` is a NUL-terminated UTF-8 JSON object:
 * `{ "snapshot": { ... }, "alerts": [ ... ] }`
 * The pointer is valid only for the duration of the callback; copy if needed.
 */
typedef void (*WbTrayUpdateCallback)(const char *update_json_utf8, void *user_data);

/**
 * Create and start the tray service and background poller.
 * Returns NULL if the underlying HID/OS transport or poller thread fails to initialize.
 */
WbTrayHandle *wb_tray_create(WbTrayUpdateCallback callback, void *user_data);

/** Stop background polling and free the handle. Safe to call with NULL. */
void wb_tray_destroy(WbTrayHandle *handle);

/** Inform the service whether the host OS UI language resolves to Simplified Chinese. */
void wb_tray_set_system_language_zh(const WbTrayHandle *handle, bool is_zh);

/**
 * Return the latest snapshot as a newly allocated UTF-8 JSON string.
 * Caller must release the returned pointer with `wb_tray_string_free`.
 */
char *wb_tray_snapshot_json(const WbTrayHandle *handle);

/** Trigger an asynchronous full refresh of all eligible devices. */
void wb_tray_request_refresh(const WbTrayHandle *handle);

/** Trigger an asynchronous refresh of a single device by opaque session ID. */
void wb_tray_request_device_refresh(const WbTrayHandle *handle, const char *device_id_utf8);

/**
 * Replace and persist configuration from a UTF-8 JSON `TrayConfig` payload.
 * Returns true if the JSON was valid and queued; updated snapshots emit on the poller thread.
 */
bool wb_tray_update_config_json(const WbTrayHandle *handle, const char *config_json_utf8);

/**
 * Return sanitized passive diagnostic JSON for `device_id_utf8`, or NULL on error.
 * Caller must release non-NULL pointers with `wb_tray_string_free`.
 */
char *wb_tray_diagnose_json(const WbTrayHandle *handle, const char *device_id_utf8);

/**
 * Return sanitized error report JSON (combining the latest read error kind and
 * passive diagnostic metadata) for `device_id_utf8`, or NULL on error.
 * Caller must release non-NULL pointers with `wb_tray_string_free`.
 */
char *wb_tray_error_report_json(const WbTrayHandle *handle, const char *device_id_utf8);

/** Check whether an external update URL starts with `https://github.com/`. */
bool wb_tray_is_valid_update_url(const char *url_utf8);

/**
 * Return the GitHub Releases latest API URL as a newly allocated UTF-8 string.
 * Caller must release the returned pointer with `wb_tray_string_free`.
 */
char *wb_tray_latest_release_api_url(void);

/**
 * Return the GitHub Releases fallback download page URL as a newly allocated UTF-8 string.
 * Caller must release the returned pointer with `wb_tray_string_free`.
 */
char *wb_tray_releases_url(void);

/**
 * Evaluate a GitHub Releases latest JSON response against `current_version_utf8`.
 * Returns a JSON string `{ "status": "available"|"up_to_date"|"failed", ... }`.
 * Caller must release the returned pointer with `wb_tray_string_free`.
 */
char *wb_tray_evaluate_release_json(const char *body_json_utf8, const char *current_version_utf8);

/**
 * Format an optional RFC3339 timestamp relative to now (`iso8601_utf8` may be NULL or empty).
 * Caller must release the returned pointer with `wb_tray_string_free`.
 */
char *wb_tray_format_relative_time(const char *iso8601_utf8, bool is_zh);

/**
 * Format a polling interval in seconds for Settings UI labels (supporting custom values).
 * Caller must release the returned pointer with `wb_tray_string_free`.
 */
char *wb_tray_format_poll_label(uint64_t secs, bool is_zh);

/**
 * Format the footer summary (`version · sync · cadence`).
 * Caller must release the returned pointer with `wb_tray_string_free`.
 */
char *wb_tray_format_footer_summary(
    const char *app_version_utf8,
    const char *last_sync_iso8601_utf8,
    uint64_t poll_secs,
    bool is_zh
);

/** Release any UTF-8 string allocated by this library. Safe to call with NULL. */
void wb_tray_string_free(char *ptr);

/*
 * Development-only symbols exported ONLY when `wireless-battery-tray-ffi` is
 * built with `--features dev-tools`. Do not call from release builds.
 */
bool wb_tray_dev_toggle_low_battery(const WbTrayHandle *handle);
void wb_tray_dev_send_low_battery_alert(const WbTrayHandle *handle, uint8_t percentage);
bool wb_tray_dev_add_device_json(const WbTrayHandle *handle, const char *spec_json_utf8);
void wb_tray_dev_reset(const WbTrayHandle *handle);

#ifdef __cplusplus
}
#endif

#endif /* WIRELESS_BATTERY_TRAY_H */
