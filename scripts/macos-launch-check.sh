#!/usr/bin/env bash
set -uo pipefail

# ── macOS launch check ──
# Usage: ./scripts/macos-launch-check.sh <path/to/Sumi.app> <ok|fail> [grace_seconds]
#
# Launches the app through LaunchServices (`open`, like Finder or Raycast),
# waits, and checks whether its process is still alive. Prints the `open`
# error and the AMFI / taskgated system log lines so a failure such as
# "Unsatisfied entitlements" (issue #39) is visible in CI output.
# Exits 0 when the result matches the expectation.

if [ $# -lt 2 ] || [[ ! "$2" =~ ^(ok|fail)$ ]]; then
  echo "Usage: $0 <path/to/App.app> <ok|fail> [grace_seconds]"
  exit 2
fi
APP="$(cd "$1" && pwd)"
EXPECT="$2"
GRACE="${3:-15}"
EXE_NAME="$(plutil -extract CFBundleExecutable raw "$APP/Contents/Info.plist")"
EXE="$APP/Contents/MacOS/$EXE_NAME"

pkill -x "$EXE_NAME" 2>/dev/null && sleep 2
SINCE="$(date '+%Y-%m-%d %H:%M:%S')"

OPEN_OUT="$(open -n "$APP" 2>&1)"
OPEN_RC=$?
sleep "$GRACE"

if pgrep -f "$EXE" >/dev/null; then RESULT=ok; else RESULT=fail; fi

echo "open exit code: $OPEN_RC"
[ -n "$OPEN_OUT" ] && echo "open output: $OPEN_OUT"
echo "--- system log since launch (AMFI / taskgated) ---"
log show --style compact --start "$SINCE" --predicate \
  'process == "taskgated-helper" OR process == "amfid" OR eventMessage CONTAINS[c] "Unsatisfied entitlements" OR eventMessage CONTAINS[c] "No matching profile"' \
  2>/dev/null | grep -v '^Timestamp' | tail -n 15
echo "---"

pkill -x "$EXE_NAME" 2>/dev/null
echo "launch result: $RESULT (expected: $EXPECT)"
[ "$RESULT" = "$EXPECT" ]
