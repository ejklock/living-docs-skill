# Delegated exploration

A recipe for handing the search of the bundle and the code to a cheaper subagent. The session
keeps every decision and every record body; the subagent only finds and quotes.

## What to delegate

Finding which records govern a topic, and locating the code they cite:

- `living-docs read records --topic <term>` for the active records on a subject.
- `living-docs read records --links-to <TYPE/NNNN>` for who links a record.
- `living-docs read records --impact <TYPE/NNNN>` for who reaches it through a chain, with distance.
- Then search the repository for the files, symbols and tests those records name.

Give the subagent the topic, the question to answer and the commands above. Read-only
access is enough; it needs no write verb.

## What it returns

A list of findings. Each finding is:

- a reference, `TYPE/NNNN` (for example `ADR/0017`) for a record, or a repository path
  (with a line when it has one) for code;
- a verbatim quote from that record or file that supports the finding, copied exactly,
  never a paraphrase or a summary.

A finding without a reference and a verbatim quote is discarded. A record the subagent
could not find is reported as not found, never guessed.

## What the session does with it

The session reads every cited record itself before it decides anything, and checks each
quote against the record. A record the subagent did not cite is still the session's to find
when the topic points at it: the result is a shortlist, not proof that nothing else governs.
A body that opens with a `SUPERSEDED` or `DEPRECATED` callout is history; follow its
successor.

## What is never delegated

- Drafting a record body or a Decision bullet: the session writes both.
- Review: the session reviews its own drafts and any other agent's findings.
- Mechanical edits the CLI owns: frontmatter, indexes, supersede links (`living-docs write`,
  `living-docs manage`).

## Model and agent definition

The model and the agent definition come from the user's own harness configs. living-docs
names no model and ships no agent file; pick the cheapest model the harness offers that
returns exact quotes.
