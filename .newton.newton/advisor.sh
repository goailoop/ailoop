#!/bin/bash
# this should analyze output from evaluator and propose a plan of actions to take
GOAL_FILE="./.newton/goal.md"
EVALUATOR_STATUS_FILE="./.newton/state/evaluator_status.md"
EXECUTION_OUTPUT_FILE="./.newton/state/execution_output.md"
ADVISOR_RECOMMENDATIONS_FILE="./.newton/state/advisor_recommendations.md"

# Build prompt based on available files
PROMPT="Your goals are set on file $GOAL_FILE. Read content of file $EVALUATOR_STATUS_FILE"
if [ -f "$EXECUTION_OUTPUT_FILE" ]; then
    PROMPT="$PROMPT and read content of file $EXECUTION_OUTPUT_FILE which contains the latest execution output"
fi
PROMPT="$PROMPT and propose a concise plan of list of actions. Don't write any data, only answer with the proposal."
# read file content to variable and pass to opencode run
#content = $(cat "$EVALUATOR_STATUS_FILE")
echo "$PROMPT"
opencode run "$PROMPT" | tee "$ADVISOR_RECOMMENDATIONS_FILE"
