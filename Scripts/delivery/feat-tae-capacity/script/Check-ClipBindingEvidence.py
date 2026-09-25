"""Check captured target clips against the same session's resource binding.

Returns 0 for matching captures, 1 for a binding mismatch, 2 for invalid evidence.
This narrow regression predicate does not certify visual playback or events.
"""
import argparse
import json
from pathlib import Path
import sys


def check(resource, capture):
    if resource["pid"] != capture["pid"] or resource["base"] != capture["base"]:
        raise ValueError("Evidence belongs to different processes")
    if capture["errors"] or not resource["character_binding_matches_resource"]:
        raise ValueError("Evidence contains read errors or unvalidated resource binding")
    animation = resource["animation_id"]
    expected_name = f"a{animation//1000000:03d}_{animation%1000000:06d}"
    clips = [clip for change in capture["changes"] for clip in change["clips"]]
    if not clips or any(clip["name"] != expected_name for clip in clips):
        raise ValueError("Missing or wrong target clip observations")
    matches = sum(int(clip["binding"],16) == int(resource["binding"],16)
                  for clip in clips)
    return {"animation": expected_name, "pid":resource["pid"],
            "observations":len(clips), "matching_resource_binding":matches,
            "map_count":resource["map_count"],
            "map_index":resource["map_index_zero_based"],
            "observed_signed_indices":sorted({clip["signed_index"] for clip in clips}),
            "verdict":"PASS" if matches == len(clips) else "FAIL",
            "scope":"Captured binding identity only; visual/event acceptance is human."}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("resource",type=Path)
    parser.add_argument("capture",type=Path)
    args = parser.parse_args()
    try:
        result = check(json.loads(args.resource.read_text(encoding="utf-8-sig")),
                       json.loads(args.capture.read_text(encoding="utf-8-sig")))
    except (ValueError,KeyError,OSError) as error:
        print(json.dumps({"verdict":"INVALID","error":str(error)}))
        sys.exit(2)
    print(json.dumps(result,indent=2))
    sys.exit(0 if result["verdict"] == "PASS" else 1)
