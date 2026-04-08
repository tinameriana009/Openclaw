from __future__ import annotations

import json
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
DEMO_ROOT = REPO_ROOT / 'docs' / 'examples' / 'unreal-runtime-telemetry-demo'
PLUGIN_ROOT = DEMO_ROOT / 'plugin' / 'RuntimeTelemetry'
SOURCE_ROOT = PLUGIN_ROOT / 'Source' / 'RuntimeTelemetry'

REQUIRED_FILES = [
    DEMO_ROOT / 'README.md',
    DEMO_ROOT / 'brief.md',
    DEMO_ROOT / 'expected-findings.md',
    DEMO_ROOT / 'manual-validation-checklist.md',
    DEMO_ROOT / 'error-feedback-playbook.md',
    DEMO_ROOT / 'operator-session-template.md',
    DEMO_ROOT / 'next-prompt-template.md',
    DEMO_ROOT / 'trace-review-checklist.md',
    PLUGIN_ROOT / 'RuntimeTelemetry.uplugin',
    SOURCE_ROOT / 'RuntimeTelemetry.Build.cs',
    SOURCE_ROOT / 'Public' / 'RuntimeTelemetrySubsystem.h',
    SOURCE_ROOT / 'Public' / 'TelemetryBlueprintLibrary.h',
    SOURCE_ROOT / 'Private' / 'RuntimeTelemetryModule.cpp',
    SOURCE_ROOT / 'Private' / 'RuntimeTelemetrySubsystem.cpp',
    SOURCE_ROOT / 'Private' / 'TelemetryBlueprintLibrary.cpp',
    REPO_ROOT / 'docs' / 'workflows' / 'README.md',
    REPO_ROOT / 'docs' / 'workflows' / 'unreal-plugin.md',
]


def main() -> int:
    missing = [path for path in REQUIRED_FILES if not path.exists()]
    if missing:
        print('Missing Unreal demo files:')
        for path in missing:
            print(f'- {path.relative_to(REPO_ROOT)}')
        return 1

    plugin_descriptor = json.loads((PLUGIN_ROOT / 'RuntimeTelemetry.uplugin').read_text())
    module_names = [module['Name'] for module in plugin_descriptor.get('Modules', [])]
    if module_names != ['RuntimeTelemetry']:
        print('Unexpected Unreal demo module list in .uplugin.')
        return 1

    expected_text = (DEMO_ROOT / 'expected-findings.md').read_text()
    for needle in [
        'RuntimeTelemetry.Build.cs',
        'RuntimeTelemetrySubsystem.h',
        'TelemetryBlueprintLibrary.h',
        'RuntimeTelemetryModule.cpp',
        'UGameInstanceSubsystem',
    ]:
        if needle not in expected_text:
            print(f'Expected findings are missing required reference: {needle}')
            return 1

    build_text = (SOURCE_ROOT / 'RuntimeTelemetry.Build.cs').read_text()
    for needle in ['Core', 'CoreUObject', 'Engine', 'Projects']:
        if needle not in build_text:
            print(f'Build.cs is missing dependency anchor: {needle}')
            return 1

    subsystem_header = (SOURCE_ROOT / 'Public' / 'RuntimeTelemetrySubsystem.h').read_text()
    for needle in ['UGameInstanceSubsystem', 'RecordEvent', 'FlushEvents', 'GetBufferedEventCount']:
        if needle not in subsystem_header:
            print(f'Subsystem header is missing expected symbol: {needle}')
            return 1

    subsystem_cpp = (SOURCE_ROOT / 'Private' / 'RuntimeTelemetrySubsystem.cpp').read_text()
    if 'Real export paths and durability guarantees need project-specific validation.' not in subsystem_cpp:
        print('Subsystem implementation does not preserve the conservative validation note.')
        return 1

    readme_text = (DEMO_ROOT / 'README.md').read_text()
    for needle, message in [
        ('python3 tests/validate_unreal_demo.py', 'Unreal demo README does not explain how to run validation.'),
        ('prepare-unreal-demo.sh', 'Unreal demo README does not mention the prep helper.'),
        ('.demo-artifacts/unreal-demo/', 'Unreal demo README does not mention the staged artifact path.'),
        ('error-feedback-playbook.md', 'Unreal demo README does not mention the error feedback playbook.'),
        ('operator-session-template.md', 'Unreal demo README does not mention the operator session template.'),
        ('next-prompt-template.md', 'Unreal demo README does not mention the follow-up prompt template.'),
        ('bundle-summary.json', 'Unreal demo README does not mention the staged bundle metadata file.'),
        ('bundle-checksums.txt', 'Unreal demo README does not mention the staged checksum file.'),
    ]:
        if needle not in readme_text:
            print(message)
            return 1

    rust_readme_text = (REPO_ROOT / 'rust' / 'README.md').read_text()
    if './scripts/prepare-unreal-demo.sh' not in rust_readme_text:
        print('rust/README.md does not mention the Unreal demo prep helper.')
        return 1

    workflow_text = (REPO_ROOT / 'docs' / 'workflows' / 'unreal-plugin.md').read_text()
    for needle in [
        'unreal-runtime-telemetry-demo',
        'manual-validation-checklist.md',
        'error-feedback-playbook.md',
        'operator-session-template.md',
        'next-prompt-template.md',
        'trace-review-checklist.md',
        'python3 tests/validate_unreal_demo.py',
    ]:
        if needle not in workflow_text:
            print(f'Unreal workflow does not mention required demo asset: {needle}')
            return 1

    index_text = (REPO_ROOT / 'docs' / 'workflows' / 'README.md').read_text().lower()
    if 'unreal runtime telemetry demo kit' not in index_text:
        print('Workflow index does not mention the Unreal runtime telemetry demo kit.')
        return 1

    print('Unreal demo validation passed.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
