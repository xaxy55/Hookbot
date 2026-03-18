#!/usr/bin/env python3
"""Extract screenshot attachments from an .xcresult bundle.

Uses the modern xcresulttool API (Xcode 16+).
"""

import json
import subprocess
import sys
import os


def run(cmd):
    return subprocess.check_output(cmd, shell=True).decode()


def main():
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <xcresult-path> <output-dir>")
        sys.exit(1)

    xcresult = sys.argv[1]
    output_dir = sys.argv[2]
    os.makedirs(output_dir, exist_ok=True)

    # Step 1: List all tests
    tests_json = run(
        f"xcrun xcresulttool get test-results tests --path '{xcresult}'"
    )
    tests = json.loads(tests_json)

    # Collect test IDs from leaf nodes (Test Case)
    test_ids = []
    def walk_nodes(nodes):
        for node in nodes:
            if node.get("nodeType") == "Test Case" and node.get("result") == "Passed":
                test_ids.append(node["nodeIdentifier"])
            if "children" in node:
                walk_nodes(node["children"])

    walk_nodes(tests.get("testNodes", []))

    if not test_ids:
        print("No passed tests found.")
        sys.exit(1)

    print(f"Found {len(test_ids)} test(s) with screenshots")

    # Step 2: For each test, get activities and extract attachments
    count = 0
    for test_id in test_ids:
        activities_json = run(
            f"xcrun xcresulttool get test-results activities "
            f"--path '{xcresult}' --test-id '{test_id}'"
        )
        activities = json.loads(activities_json)

        for test_run in activities.get("testRuns", []):
            for activity in test_run.get("activities", []):
                count += extract_attachments(activity, xcresult, output_dir, count)

    if count == 0:
        print("No screenshots found in xcresult bundle.")
        sys.exit(1)
    else:
        print(f"\nExtracted {count} screenshots to {output_dir}/")


def extract_attachments(activity, xcresult, output_dir, count):
    """Recursively extract attachments from an activity tree."""
    extracted = 0

    for attachment in activity.get("attachments", []):
        payload_id = attachment.get("payloadId", "")
        raw_name = attachment.get("name", f"screenshot_{count + extracted}")

        # Clean up the name: "01_idle_0_UUID.png" -> "01_idle"
        name = raw_name
        # Remove UUID suffix and extension
        if "_" in name:
            parts = name.rsplit("_", 2)
            if len(parts) >= 3:
                # e.g., 01_idle_0_UUID.png -> 01_idle
                name = "_".join(parts[:-2])
        name = name.replace(".png", "")

        if not payload_id:
            continue

        out_path = os.path.join(output_dir, f"{name}.png")
        subprocess.run(
            f"xcrun xcresulttool export --path '{xcresult}' "
            f"--id '{payload_id}' --output-path '{out_path}' "
            f"--type file",
            shell=True,
            capture_output=True,
        )

        if os.path.exists(out_path) and os.path.getsize(out_path) > 0:
            extracted += 1
            size_kb = os.path.getsize(out_path) // 1024
            print(f"  ✓ {out_path} ({size_kb} KB)")
        else:
            # Fallback: try legacy export
            subprocess.run(
                f"xcrun xcresulttool get object --legacy "
                f"--path '{xcresult}' --id '{payload_id}' "
                f"> '{out_path}'",
                shell=True,
                capture_output=True,
            )
            if os.path.exists(out_path) and os.path.getsize(out_path) > 0:
                extracted += 1
                size_kb = os.path.getsize(out_path) // 1024
                print(f"  ✓ {out_path} ({size_kb} KB) [legacy]")
            else:
                print(f"  ✗ Failed to export: {raw_name}")

    # Recurse into child activities
    for child in activity.get("childActivities", []):
        extracted += extract_attachments(child, xcresult, output_dir, count + extracted)

    return extracted


if __name__ == "__main__":
    main()
