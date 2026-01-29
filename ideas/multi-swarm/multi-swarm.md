# Willow Rambles About Hiveminds

## The SWARM

So lately I have been doing an awful lot of programming with AI. There's an awful lot to say about this, but one of the most interesting things to me is the ability for an AI agent to create new AI agents. For example, you might be talking to a coding agent that wants to understand some software without filling up its context window. So it can create a new agent that explores the codebase for it, and then provides a summary back to the agent. We'll call that secondary agent a *subagent*

This gets more interesting once you get into swarms. To me, a swarm is a whole team of subagents that work together to complete a goal. In order to work together, they have access to a shared file which they can use to communicate with each other. By editing the file, they can ask each other questions. Then when they're all done, they can all write up reports on their work and then the main agent can review these reports.

Swarms like this are a really powerful tool, because it means you can do so much more in less time. Standard division of labour type shit - 10 people can do more than 1 in the same amount of time. I use them a lot


## Managing Multiple Swarms

A natural progression then comes along: why one swarm when you can have two? Three? Ten? Each swarm could work on different features on a project at the same time, a whole army of teams working on one piece of software.

The problem that comes from this is: how the fuck do you manage all of that? How do we prevent swarms clobbering each other's work?

Well, that one is easy. Just get them to work on different git branches. Since they're working in the same filesystem, you can assign git worktrees to each different branch. So each swarm can work on its own collection of files, version controlled in git branches and backed up to github or whatever.

## Groupchats and git

The problem with this worktree approach and swarms is figuring out how to track the hive mind files. Keeping copies of reports and hive-mind files is really useful, because it explains what work was done. However, each swarm working on different branches creates fragmentation. Different branches get different sets of reports, which can be a problem if you want one swarm to be aware of what another swarm did.

What we ideally want is a scratchpad folder at the top of the repo, which contains all the reports and hive mind files from agents. This folder should be easily accessible by all agents, by being shared between all the branches. So every branch should see the exact same scratchpad.

The problem is that this is directly against how git works. Branches are deliberately not supposed to share state. The easy solution would be to not put the scratchpad in the repo, however that gets rid of the easy version controlling and backups which you get with using git repos.

## My janky ass idea

Ok so my solution is to basically have a dedicated scratchpad branch. This branch is completely empty of all files, except for the contents of the scratchpad. No other branch contains these files.

This branch then gets mounted alongside all the others as a git worktree. This gives all swarms access to the same scratchpad branch, but by design of the hive mind system they'll never touch each other's files. Then the overseer agent which manages all the swarms can handle the pushing and pulling of the scratchpad branch.

