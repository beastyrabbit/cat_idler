# Private native review saves

Create a new isolated save for normal native UI and performance checks:

```sh
dotnet run --project tools/presentation-fixture/Forest.PresentationFixture.csproj -- /absolute/new/founding.json 30
dotnet run --project tools/presentation-fixture/Forest.PresentationFixture.csproj -- /absolute/new/expanded.json 150
```

Pass the created path to the native app with `--forest-save`. The tool refuses an existing save or identity directory, uses `LocalAuthority.CreateNew`, validates, saves, and reloads the result. It never prints credentials. Its adjacent `.identity` directory is private local authentication state and must not be published.

The 30-cat mode retains the maintained founding blueprint and adds finite review supplies, 20,000 research points, and 200 blessings before play. The 150-cat mode creates 30 Dens, all building types, finite goods, a known radius-23 enclosure, exterior resource sources, and staffed repeating production chains. It advances 15 seconds so the saved workers already have real jobs. Enough research remains unowned for ordinary research purchases.

This is a test-only command. It adds no developer bypass to the game or server and does not migrate or overwrite an existing save.
