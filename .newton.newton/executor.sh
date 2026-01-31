# we want to capture output to a file but also show output on the terminal
GOAL_FILE="./.newton/goal.md"
ADVISOR_RECOMMENDATIONS_FILE="./.newton/state/advisor_recommendations.md"
EXECUTION_OUTPUT_FILE="./.newton/state/execution_output.md"
PROJECT_FOLDER="$(pwd)"
PROMPT="You are working on project located at $PROJECT_FOLDER. Your goals are set on file $GOAL_FILE. Read content of file $ADVISOR_RECOMMENDATIONS_FILE which contains important recomendations on how to make progress on the issue. "
echo "$PROMPT"
opencode run "$PROMPT" | tee "$EXECUTION_OUTPUT_FILE"
