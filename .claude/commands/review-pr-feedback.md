---
description: Review PR feedback comments on the current branch and resolve actionable items
---

# Review PR Feedback

Review all feedback comments on the pull request associated with the current branch and resolve any actionable items.

## Steps

1. **Identify the current branch and its PR:**
   - Run `git branch --show-current` to get the current branch name.
   - Run `gh pr view --json number,title,url,state` to get the PR associated with this branch. If no PR exists, stop and inform the user.

2. **Retrieve all review comments and feedback:**
   - Run `gh pr view --json reviews,comments` to get top-level PR comments and reviews.
   - Run `gh api repos/{owner}/{repo}/pulls/{number}/comments` to get all inline review comments (diff comments). Parse the JSON to extract: `path`, `line` (or `original_line`), `body`, `user.login`, `diff_hunk`, `created_at`, and `in_reply_to_id`.
   - Run `gh pr view --json reviewDecision,reviewRequests,latestReviews` to understand the overall review status.
   - Get the owner/repo from `gh repo view --json nameWithOwner --jq .nameWithOwner`.

3. **Organize and present the feedback:**
   - Group comments by file path.
   - For threaded conversations (comments with `in_reply_to_id`), nest replies under their parent.
   - Clearly distinguish between:
     - **Actionable feedback**: Requests for code changes, bug reports, suggestions for improvement.
     - **Questions**: Clarification requests that may need a response rather than a code change.
     - **Non-actionable**: Approvals, acknowledgments, "nit" comments that are optional, praise.
   - Present a summary of all feedback to the user before making any changes.

4. **Resolve actionable feedback:**
   - For each actionable comment, read the relevant file and understand the context using the `diff_hunk` and `path` from the comment.
   - Implement the requested fix or improvement.
   - After making changes, briefly summarize what was done for each item.

5. **Report unresolvable items:**
   - If any feedback cannot be addressed automatically (e.g., architectural questions, ambiguous requests, questions needing human input), list them clearly so the user can follow up manually.
