"""Strict separate opt-in ordinary Preview telemetry selector; no artifact IO."""
ENV = 'NATIVE_HOME_PREVIEW_PASSIVE_PERFORMANCE'
NAME = 'preview-performance-observer.json'
def select(environ, *, main, target, app_id, test_identity, interactive):
    if ENV not in environ:
        return None
    if environ[ENV] != '1':
        raise ValueError('Passive performance mode must be explicitly 1')
    allowed = {'NATIVE_HOME_BUILD_DIR','NATIVE_HOME_WEBOS_SDK','NATIVE_HOME_ARM_BINARY',
      'NATIVE_HOME_CACHE_PRODUCER_BROKER','NATIVE_HOME_SNAPSHOT_ASSETS','NATIVE_HOME_PREVIEW_ICON',
      'NATIVE_HOME_CARGO','NATIVE_HOME_RUST_TOOLCHAIN','NATIVE_HOME_INTERACTIVE_SESSION', ENV,
      'NATIVE_HOME_CACHE_CONSUMER_COMPATIBILITY','NATIVE_HOME_CACHE_CONSUMER_OLD_UI',
      'NATIVE_HOME_CACHE_CONSUMER_OLD_BROKER','NATIVE_HOME_CACHE_CONSUMER_OLD_PROFILE'}
    if (main != 'home-preview' or target != 'arm' or app_id != 'com.ash.nativehome.preview'
      or test_identity or not interactive or environ.get('NATIVE_HOME_INTERACTIVE_SESSION') != '1'
      or not environ.get('NATIVE_HOME_CACHE_PRODUCER_BROKER')
      or any(k.startswith('NATIVE_HOME_') and k not in allowed for k in environ)):
        raise ValueError('Passive performance requires only ordinary interactive ARM Preview and normal cache provenance')
    return {'schema':2,'lifetime_seconds':240,'snapshot_interval_ms':1000}
