### Router Report
LLM router dump report for prompt: {{prompt}}

| Domain | Operation |
|--------|-----------|
{%for action in actions%}| {{action.domain}} | {{action.operation}} |
{%endfor%}

Note: Routing decission was performed in {{duration}} seconds with a confidence of {{confidence}}.