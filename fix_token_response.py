#!/usr/bin/env python3
"""Update TokenResponse schema and examples per Story 2.3."""
import yaml
import re

SPEC_PATH = 'openapi/idam/identity-login-service/openapi.yaml'

with open(SPEC_PATH) as f:
    data = yaml.safe_load(f)

# Update TokenResponse schema
tr = data['components']['schemas']['TokenResponse']
# Remove PII fields
for field in ['email', 'email_verified', 'phone_verified']:
    if field in tr['properties']:
        del tr['properties'][field]
    # Also remove from required if present
    if field in tr.get('required', []):
        tr['required'].remove(field)

# Add new fields
tr['properties']['entitlements_ref'] = {
    'type': 'string',
    'description': 'Deterministic reference to the user\'s entitlements snapshot stored in Redis. Format: ent_<uuid_v5>. Consumers use this as a cache key to fetch the full ACL snapshot.'
}
tr['properties']['entitlements_hash'] = {
    'type': 'string',
    'description': 'SHA-256 hash of the canonical JSON representation of the entitlements snapshot. Used by consumers to verify snapshot integrity after fetching from Redis cache.'
}
tr['properties']['ver'] = {
    'type': 'integer',
    'description': 'Authorization version counter. Bumped on permission changes to invalidate stale tokens.'
}

print("TokenResponse schema updated:")
for k, v in tr['properties'].items():
    print(f"  {k}: {v.get('description', '')[:80]}")

# Now update all examples in operations that reference PII fields
def clean_value(val):
    """Remove PII fields from example values."""
    if isinstance(val, dict):
        return {k: clean_value(v) for k, v in val.items() if k not in ('email', 'email_verified', 'phone_verified')}
    elif isinstance(val, list):
        return [clean_value(v) for v in val]
    return val

def fix_examples(node, depth=0):
    """Recursively fix examples in responses."""
    if depth > 50:
        return
    if isinstance(node, dict):
        if 'value' in node and isinstance(node['value'], dict):
            node['value'] = clean_value(node['value'])
        for k, v in node.items():
            fix_examples(v, depth + 1)
    elif isinstance(node, list):
        for item in node:
            fix_examples(item, depth + 1)

# Fix all operation examples
for path, path_item in data.get('paths', {}).items():
    for method, op in path_item.items():
        if isinstance(op, dict) and 'responses' in op:
            for resp_code, resp in op['responses'].items():
                if isinstance(resp, dict) and 'content' in resp:
                    for ct, ct_data in resp['content'].items():
                        if 'examples' in ct_data:
                            fix_examples(ct_data['examples'])
                        if 'example' in ct_data:
                            ct_data['example'] = clean_value(ct_data['example'])

# Also update the schema-level example
if 'example' in tr:
    tr['example'] = clean_value(tr['example'])

# Add entitlements fields to the example
if isinstance(tr.get('example'), dict):
    tr['example']['entitlements_ref'] = 'ent_550e8400-e29b-41d4-a716-446655440000'
    tr['example']['entitlements_hash'] = 'sha256:a1b2c3d4e5f6...'
    tr['example']['ver'] = 1

with open(SPEC_PATH, 'w') as f:
    yaml.dump(data, f, sort_keys=False, default_flow_style=False, width=120)

print("\nDone! All PII references cleaned from examples.")
