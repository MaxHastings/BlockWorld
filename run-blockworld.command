#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")"

if ! command -v mvn >/dev/null 2>&1; then
	echo "BlockWorld needs Maven to download its Java dependencies."
	read -r -p "Press Return to close..."
	exit 1
fi

JAVA_RUNTIME=$(/usr/libexec/java_home -a arm64 -v '17+' 2>/dev/null || true)
if [[ ! -x "$JAVA_RUNTIME/bin/java" || ! -x "$JAVA_RUNTIME/bin/javac" ]]; then
	echo "BlockWorld needs an ARM Java 17 or newer JDK."
	read -r -p "Press Return to close..."
	exit 1
fi

if ! mvn -q -DskipTests package dependency:build-classpath -Dmdep.outputFile=target/classpath.txt; then
	echo "BlockWorld could not be built. Check the Maven output above."
	read -r -p "Press Return to close..."
	exit 1
fi

CLASSPATH="target/classes:$(cat target/classpath.txt)"
exec "$JAVA_RUNTIME/bin/java" -XstartOnFirstThread -cp "$CLASSPATH" Blockworld.Screen "$@"
