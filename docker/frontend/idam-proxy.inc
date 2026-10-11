# Shared proxy settings for the /idam/ locations in nginx.conf.
proxy_set_header Host $host;
proxy_set_header X-Real-IP $remote_addr;
proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
proxy_set_header X-Forwarded-Proto $scheme;
# Service keys never come from a browser: drop them (ApiKeyHeader, PlatformServiceAuth).
proxy_set_header X-API-KEY "";
proxy_set_header X-Platform-Admin-Key "";
