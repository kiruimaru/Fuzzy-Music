# 0004: Immutable Windows child network policy

The desktop shell snapshots network policy at startup and supplies the same typed
`NetworkPolicy` to yt-dlp and mpv via explicit `Command` environment entries.
Restart the app after changing Windows proxy settings. No parent environment,
registry, routes, VPN configuration, PATH or system settings are modified.

`WinHttpGetIEProxyConfigForCurrentUser` reads the current interactive user's
Internet Options (including the enabled manual proxy), unlike service-level
WinHTTP defaults. Its UTF-16 buffers are freed with GlobalFree. The implementation
uses existing Windows libraries without adding dependencies. The reusable policy
lives alongside the lowest-level child adapter, and is re-exported by application
for the shell/player; existing source constructors retain inherited-env behavior.

Precedence:

1. Any explicit HTTP_PROXY, HTTPS_PROXY or ALL_PROXY, including empty values,
   selects the complete inherited environment policy. Windows names are matched
   case-insensitively. No missing protocol is filled from system configuration.
   Tool-native protocol/fallback semantics apply; NO_PROXY is preserved.
2. Otherwise enabled manual Windows proxy: `host:port`, `http://host:port`, or
   `http=host:port;https=host:port`. HTTPS mapping means HTTP CONNECT for HTTPS
   targets, not a TLS connection to the proxy itself. Unsupported mappings
   (including SOCKS) produce a fixed diagnostic and direct routing.
3. Disabled system proxy means normal OS routing, including VPN TUN routes.
   A listening local proxy alone is not discoverable without system config/env.

Loopback bypass is always added for system policy. Semicolon bypass entries and
leading `*.` suffixes are translated to NO_PROXY; explicit NO_PROXY overrides this
list. Windows `<local>` unqualified-name matching and arbitrary wildcard/CIDR
semantics cannot be represented portably by both tools; only loopback and simple
host/domain bypass are supported. PAC/WPAD is not evaluated: automatic-only
configuration produces a fixed unsupported/direct diagnostic; manual settings,
if also present, take precedence. This limitation is not reported as successful
automatic proxy resolution. Query errors likewise produce a fixed diagnostic.

Only a fixed policy description is logged; Debug never exposes proxy URLs or
credentials. Managed source requests suppress raw success stderr; application
errors continue discarding extractor diagnostics. Do not log Command/environment
contents. No manual proxy UI is included. Runtime yt-dlp/mpv distribution remains
an independent packaging decision; this does not establish installer readiness.
