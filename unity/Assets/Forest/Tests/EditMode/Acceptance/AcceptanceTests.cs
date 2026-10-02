using System;
using System.Collections;
using IdleCatForest.Acceptance;
using NUnit.Framework;

public class AcceptanceTests
{
    public static IEnumerable Scenarios()
    {
        foreach (var c in AcceptanceScenarios.Cases())
        {
            var data = new TestCaseData(c.Name).SetName(c.Name);
            // Campaign twins run for minutes each; Run All must never pick them up.
            if (c.Name.StartsWith("campaign.", StringComparison.Ordinal))
                data = data.SetCategory("Campaign").Explicit("campaign");
            yield return data;
        }
    }

    [TestCaseSource(nameof(Scenarios))]
    public void RequiredOutcome(string name)
    {
        AcceptanceScenarios.Run(name);
    }
}
