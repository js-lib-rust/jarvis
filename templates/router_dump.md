### Router Report
LLM router dump report for prompt: {{prompt}}

{%for action in actions%}
#### Service: `{action.domain}`
{{action.operation}}
{%endfor%}

#### Service: `user-profile`
- Get my username.
- Get the birth date of {username}.

#### Service: `time-service`
- Get date for today.
- Compute age as the difference in years between {date} and {birth_date}.

#### Service: `printer`
- Print computed age in years for {username}.