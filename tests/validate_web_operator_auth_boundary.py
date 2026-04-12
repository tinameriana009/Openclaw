from __future__ import annotations

import json
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
RUST_ROOT = REPO_ROOT / 'rust'
DOC_PATH = RUST_ROOT / 'docs' / 'WEB_OPERATOR_AUTH_BOUNDARY.md'
SCHEMA_PATH = RUST_ROOT / 'docs' / 'schemas' / 'web-operator-auth-policy.schema.json'
EXAMPLE_PATH = RUST_ROOT / 'config-examples' / 'web-operator-auth-policy.example.json'


REQUIRED_DOC_PHRASES = [
    'does **not** currently ship a real authenticated web operator backend',
    'Disabled by default',
    'Reverse-proxy auth first',
    'Read-only before write-capable',
    'no direct internet exposure',
    'web-shaped does not mean web-safe',
]


def fail(message: str) -> int:
    print(message)
    return 1


def main() -> int:
    for path in [DOC_PATH, SCHEMA_PATH, EXAMPLE_PATH]:
        if not path.exists():
            return fail(f'Missing web auth boundary file: {path.relative_to(REPO_ROOT)}')

    doc_text = DOC_PATH.read_text()
    schema = json.loads(SCHEMA_PATH.read_text())
    example = json.loads(EXAMPLE_PATH.read_text())

    for phrase in REQUIRED_DOC_PHRASES:
        if phrase not in doc_text:
            return fail(f'WEB_OPERATOR_AUTH_BOUNDARY.md is missing required honesty/boundary phrase: {phrase}')

    if schema.get('properties', {}).get('policyKind', {}).get('const') != 'claw.web-operator-auth-boundary':
        return fail('Schema policyKind must be claw.web-operator-auth-boundary')
    if schema.get('properties', {}).get('schemaVersion', {}).get('const') != 1:
        return fail('Schema schemaVersion must be const 1')
    if schema.get('properties', {}).get('directInternetExposureAllowed', {}).get('const') is not False:
        return fail('Schema must forbid direct internet exposure')
    if schema.get('properties', {}).get('anonymousReadAllowed', {}).get('const') is not False:
        return fail('Schema must forbid anonymous reads')
    if schema.get('properties', {}).get('sessionCookiesSupported', {}).get('const') is not False:
        return fail('Schema must forbid browser session cookies for now')

    proxy = schema.get('properties', {}).get('trustedProxy', {}).get('properties', {})
    if proxy.get('allowClientSuppliedIdentityHeaders', {}).get('const') is not False:
        return fail('Schema must forbid client-supplied identity headers')

    if example.get('policyKind') != 'claw.web-operator-auth-boundary':
        return fail('Example policyKind is incorrect')
    if example.get('schemaVersion') != 1:
        return fail('Example schemaVersion is incorrect')
    if example.get('backendEnabled') is not False:
        return fail('Example must keep backendEnabled=false')
    if example.get('deploymentMode') != 'static-only':
        return fail('Example must use static-only deployment mode')
    for key in ['anonymousReadAllowed', 'mutationRoutesEnabled', 'sessionCookiesSupported', 'directInternetExposureAllowed']:
        if example.get(key) is not False:
            return fail(f'Example must keep {key}=false')

    trusted_proxy = example.get('trustedProxy')
    if not isinstance(trusted_proxy, dict):
        return fail('Example trustedProxy must be an object')
    if trusted_proxy.get('required') is not True:
        return fail('Example trustedProxy.required must be true')
    headers = trusted_proxy.get('identityHeaders')
    if not isinstance(headers, list) or not headers:
        return fail('Example trustedProxy.identityHeaders must be a non-empty list')
    if trusted_proxy.get('allowClientSuppliedIdentityHeaders') is not False:
        return fail('Example must forbid client-supplied identity headers')

    notes = example.get('notes')
    if not isinstance(notes, list) or len(notes) < 2:
        return fail('Example notes must include at least two boundary reminders')

    print('Web operator auth boundary validation passed.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
