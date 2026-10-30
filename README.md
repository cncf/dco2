# DCO2

**DCO2** is a GitHub App that enforces the [Developer Certificate of Origin](https://developercertificate.org/) (DCO) on Pull Requests.

It aims to be a drop-in replacement for [dcoapp/app](https://github.com/dcoapp/app).

## Usage

To start using the DCO2 application, you need to [configure the application](https://github.com/apps/dco-2) for your organization or repositories. To enforce the DCO check, you can enable [required status checks](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches) in your branch protection settings.

## How it works

The [Developer Certificate of Origin](https://developercertificate.org) (DCO) is a lightweight way for contributors to certify that they wrote or otherwise have the right to submit the code they are contributing to the project.

Contributors *sign-off* that they adhere to [these requirements](https://developercertificate.org) by adding a `Signed-off-by` line to commit messages.

```text
Commit message

Signed-off-by: User1 <user1@email.test>
```

Git includes a `-s` command line option to append this line automatically to your commit message (provided you have configured your `user.name` and `user.email` in your git configuration):

```text
git commit -s -m 'Commit message'
```

Once installed, this application will create a check indicating whether or not all commits in a Pull Request contain a valid `Signed-off-by` line.

![check-passed](docs/screenshots/check-passed.png)

More information about the check result is available on the check details page. This information includes a full list of the commits checked (including the reason why each of them passed or failed the check), some additional details about each error found and some instructions on how to fix them.

*(check passed details)*

![check-passed-details](docs/screenshots/check-passed-details.png)

*(check failed details)*

![check-failed-details](docs/screenshots/check-failed-details.png)

### Remediation commits

Remediation commits allow applying a sign-off *retroactively* to one or more commits that failed the DCO check.

The primary benefit of using remediation commits to fix missing or invalid sign-offs, is that the repository's history does not change, and there is no risk of breaking someone else's work.

Remediation commits can be `individual` or `third-party`, and they **must** be properly signed off.

By default, both are **disabled**, but they can be enabled in the [configuration file](docs/config/dco.yml) (`.github/dco.yml`):

```yaml
allowRemediationCommits:
  individual: true
  thirdParty: true
```

> [!NOTE]
> The DCO2 configuration file is backwards compatible with the [dcoapp/app](https://github.com/dcoapp/app) configuration file. Note that third-party remediation requires individual remediation to be enabled in the configuration as well.

#### Individual

Individual remediation commits **must** be authored by the same person who authored the failing commit.

An individual remediation commit to add a sign-off to two commits created by `User1` would look like this:

```text
DCO remediation commit for User1 <user1@email.test>

I, User1 <user1@email.test>, hereby add my Signed-off-by to this commit: <COMMIT1_SHA>
I, User1 <user1@email.test>, hereby add my Signed-off-by to this commit: <COMMIT2_SHA>

Signed-off-by: User1 <user1@email.test>
```

#### Third-party

Third-party remediation commits **must** be authored by an authorized individual on behalf of the failing commit's author.

> [!IMPORTANT]
> If you are about to add a third-party DCO remediation commit under DCO section (b) or (c), be sure you are authorized by your employer to take this action. Generally speaking, maintainers and other project contributors cannot sign off on behalf of project contributors, unless there is some relationship which permits this action. It is your responsibility to verify this.

A third-party remediation commit to add a sign-off to two commits created by `User1` would look like this:

```text
Third-party DCO remediation commit for User1 <user1@email.test>

On behalf of User1 <user1@email.test>, I, user2 <user2@email.test>, hereby add my Signed-off-by to this commit: <COMMIT1_SHA>
On behalf of User1 <user1@email.test>, I, user2 <user2@email.test>, hereby add my Signed-off-by to this commit: <COMMIT2_SHA>

Signed-off-by: User2 <user2@email.test>
```

### Skipping sign-off for organization members

It is possible to disable the check for commits authored and [signed](https://help.github.com/articles/signing-commits-using-gpg/) by members of the organization that owns the repository.

When using the public DCO2 service, _it may fail to detect that an organization member is a member._
Determining organization membership is only possible if DCO2 is able to confirm the author's membership:
the author's membership in the organization must be visible to DCO2,
either because the author has [publicized their membership](https://docs.github.com/en/account-and-profile/how-tos/organization-membership/publicizing-or-hiding-organization-membership)
or because DCO2 is configured with credentials that allow it to see non-public organization members.

By default, this feature is **disabled**. To enable it, the following section must be added to the [configuration file](docs/config/dco.yml) (`.github/dco.yml`):

```yaml
require:
  members: false
```

When this setting is present on a repository that belongs to a single user (instead of an organization), only the repository owner is allowed to push commits without a sign-off.

### Override action

By default, failed DCO checks include a `Set DCO to pass` button in the check details page.

To hide this button, set the following option in the [configuration file](docs/config/dco.yml) (`.github/dco.yml`):

```yaml
allowOverrideAction: false
```

> [!NOTE]
> This option only hides DCO2's own override action. It is not a security
> boundary because repository maintainers with write access can still change
> the repository configuration.

### Labeling pull requests

DCO2 can keep a label in sync with the DCO check result. When enabled, the label is added to open pull requests whose DCO check fails, and removed when the check passes (including when it passes thanks to remediation commits or the `Set DCO to pass` action).

By default, this feature is **disabled**. To enable it, set the name of the label to use in the [configuration file](docs/config/dco.yml) (`.github/dco.yml`):

```yaml
label: needs-dco
```

The label name can have up to 50 characters. Blank names, names longer than 50 characters, `.` and `..` are ignored, and labeling stays disabled. If the label does not exist in the repository, DCO2 creates it. Existing labels (including their colors and descriptions) are never modified.

Labeling requires the `Pull requests: write` permission (`Issues: write` can be used instead, as it also allows managing labels):

- When using the [public DCO2 application](https://github.com/apps/dco-2), the organization or account owner must approve the updated permissions requested by the application.
- When running your own instance of DCO2, add this permission to your GitHub App. No changes to the DCO2 server configuration are required.

Approving this permission is optional. DCO checks keep working without it, but DCO2 will not label pull requests. It may take up to about an hour for labeling to start after approving the permission or updating the configuration file.

Labeling is done on a best-effort basis:

- Closed pull requests are never labeled or unlabeled.
- Disabling the option does not remove labels already applied.
- When several pull requests from forks share the same head commit, the `Set DCO to pass` action only removes the label from the pull request whose check run was used. The label is removed from the other pull requests on the next push, or it can be removed manually.
- Pull requests that were already open when labeling was enabled are labeled the next time new commits are pushed to them. To label them right away, you can use the [label-open-prs.sh](docs/scripts/label-open-prs.sh) sample script, which requires bash 4 or later and the [GitHub CLI](https://cli.github.com):

  ```text
  docs/scripts/label-open-prs.sh --dry-run OWNER/REPO needs-dco
  ```

  The script labels the open pull requests whose latest DCO2 check failed. Remove `--dry-run` to apply the changes, and use `--app-slug` to set your GitHub App's slug when running your own instance of DCO2. Using the `Set DCO to pass` action on a pull request from a fork labeled by the script may not remove the label if its DCO check run does not reference the pull request; in that case, the label is removed on the next push, or it can be removed manually.

> [!NOTE]
> When several commits are pushed in very quick succession, a stale label may occasionally remain on a pull request until the next push.

## Deployment

We'll soon provide some documentation about how to deploy your own instance of DCO2.

## Contributing

Please see [CONTRIBUTING.md](./CONTRIBUTING.md) for more details.

## Code of Conduct

This project follows the [CNCF Code of Conduct](https://github.com/cncf/foundation/blob/master/code-of-conduct.md).

## License

**DCO2** is an Open Source project licensed under the [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0).
