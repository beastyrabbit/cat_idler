using System.Collections;
using IdleCatForest.Acceptance;
using NUnit.Framework;

public class AcceptanceTests
{
    public static IEnumerable Scenarios(){foreach(var c in AcceptanceScenarios.Cases())yield return new TestCaseData(c.Name).SetName(c.Name);}
    [TestCaseSource(nameof(Scenarios))]
    public void RequiredOutcome(string name){AcceptanceScenarios.Run(name);}
}
