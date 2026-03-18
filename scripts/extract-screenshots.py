#!/usr/bin/env python3
"""Extract screenshot attachments from an .xcresult bundle."""

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

    # Get the top-level result JSON
    result_json = run(f"xcrun xcresulttool get --path '{xcresult}' --format json")
    result = json.loads(result_json)

    # Navigate to test action summaries
    actions = result.get("actions", {}).get("_values", [])

    count = 0
    for action in actions:
        action_result = action.get("actionResult", {})
        tests_ref = action_result.get("testsRef", {})
        ref_id = tests_ref.get("id", {}).get("_value", "")

        if not ref_id:
            continue

        # Get test plan run summaries
        tests_json = run(f"xcrun xcresulttool get --path '{xcresult}' --format json --id '{ref_id}'")
        tests = json.loads(tests_json)

        # Walk the test summary tree to find attachments
        summaries = tests.get("summaries", {}).get("_values", [])
        for summary in summaries:
            testable_summaries = summary.get("testableSummaries", {}).get("_values", [])
            for testable in testable_summaries:
                test_classes = testable.get("tests", {}).get("_values", [])
                for test_class in test_classes:
                    subtests = test_class.get("subtests", {}).get("_values", [])
                    for subtest in subtests:
                        sub_subtests = subtest.get("subtests", {}).get("_values", [])
                        for test in sub_subtests:
                            activity_summaries = test.get("activitySummaries", {}).get("_values", [])
                            for activity in activity_summaries:
                                attachments = activity.get("attachments", {}).get("_values", [])
                                for attachment in attachments:
                                    name = attachment.get("name", {}).get("_value", f"screenshot_{count}")
                                    payload_ref = attachment.get("payloadRef", {})
                                    payload_id = payload_ref.get("id", {}).get("_value", "")

                                    if not payload_id:
                                        continue

                                    out_path = os.path.join(output_dir, f"{name}.png")
                                    subprocess.run(
                                        f"xcrun xcresulttool get --path '{xcresult}' --id '{payload_id}' > '{out_path}'",
                                        shell=True
                                    )
                                    count += 1
                                    print(f"  Saved: {out_path}")

    if count == 0:
        print("No screenshots found in xcresult bundle.")
        sys.exit(1)
    else:
        print(f"\nExtracted {count} screenshots to {output_dir}/")

if __name__ == "__main__":
    main()
