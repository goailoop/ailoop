#!/bin/bash

# Newton Loop Framework using newton CLI
# Orchestrates evaluation-advice-execution cycles

echo "Starting Newton optimization loop..."

newton run . \
  --evaluator-cmd './.newton/evaluator.sh' \
  --advisor-cmd './.newton/advisor.sh' \
  --executor-cmd './.newton/executor.sh' \
  --max-iterations 4

echo "Newton loop completed"
