# DCO2

**DCO2** is a GitHub App that enforces the [Developer Certificate of Origin](https://developercertificate.org/) (DCO) on Pull Requests.

## Usage

To start using DCO2, you need to [configure the application](https://github.com/apps/dco-2) for your organization or repositories. To enforce the DCO check, you can enable [required status checks](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches) in your branch protection settings.

## How it works

The Developer Certificate of Origin (DCO) is a lightweight way for contributors to certify that they wrote or otherwise have the right to submit the code they are contributing to the project.

Here is the full [text of the DCO](https://developercertificate.org/), reformatted for readability:

> By making a contribution to this project, I certify that:
>
> a. The contribution was created in whole or in part by me and I have the right to submit it under the open source license indicated in the file; or
>
> b. The contribution is based upon previous work that, to the best of my knowledge, is covered under an appropriate open source license and I have the right under that license to submit that work with modifications, whether created in whole or in part by me, under the same open source license (unless I am permitted to submit under a different license), as indicated in the file; or
>
> c. The contribution was provided directly to me by some other person who certified (a), (b) or (c) and I have not modified it.
>
> d. I understand and agree that this project and the contribution are public and that a record of the contribution (including all personal information I submit with it, including my sign-off) is maintained indefinitely and may be redistributed consistent with this project or the open source license(s) involved.

Contributors *sign-off* that they adhere to these requirements by adding a `Signed-off-by` line to commit messages.

```text
This is my commit message

Signed-off-by: Joe Smith <joe.smith@example.com>
```

Git includes a `-s` command line option to append this automatically to your commit message (provided you have configured your `user.name` and `user.email` in your git configuration):

```text
% git commit -s -m 'This is my commit message'
```

Once installed, this application will create a check indicating whether or not commits in a Pull Request contain a valid `Signed-off-by` line.

## Contributing

Please see [CONTRIBUTING.md](./CONTRIBUTING.md) for more details.

## Code of Conduct

This project follows the [CNCF Code of Conduct](https://github.com/cncf/foundation/blob/master/code-of-conduct.md).

## License

DCO2 is an Open Source project licensed under the [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0).
