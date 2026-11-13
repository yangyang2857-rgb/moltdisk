<!-- moltdisk:release-header -->

> **蜕冗** — `moltdisk` finds build output that can be rebuilt from source. It does not delete it.
>
> `target/`, `node_modules`, `dist/`: the tool reports the size, the idle time and the evidence behind each
> verdict, then stops. Deleting is a judgement about *your* project, made by a rulebook that has never seen
> it — and the person who pays for a wrong verdict is never the person who wrote the rule. So this binary has
> no delete path: no `remove_dir_all`, and `-y` is a hard error rather than a prompt.
>
> Verify it instead of trusting it: `strings moltdisk | grep -c remove_dir_all` prints `0`.
> The long version: [Why an auditor and not a deleter](https://github.com/yangyang2857-rgb/moltdisk#why-an-auditor-and-not-a-deleter).

---

