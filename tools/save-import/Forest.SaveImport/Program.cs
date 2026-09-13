using IdleCatForest.Authority;
using IdleCatForest.SaveImport;

if (args.Length != 2 && (args.Length != 3 || args[0] != "--credential")) { Console.Error.WriteLine("usage: Forest.SaveImport NORMALIZED.json NEW-WORLD.json | --credential OLD-SESSION.json NEW-SESSION.json"); return 2; }
try
{
    if (args.Length == 3) { CredentialStore.ImportLegacy(args[1], args[2]); Console.WriteLine("Imported native identity to a new private destination. Source was not changed."); return 0; }
    if (File.Exists(args[1])) throw new IOException("Destination exists.");
    if (new FileInfo(args[0]).Length > SaveStore.MaximumBytes) throw new InvalidDataException("Input is too large.");
    var world = LegacyImport.Convert(File.ReadAllText(args[0]));
    AuthorityRuntime.ValidateWorld(world); SaveStore.Save(args[1], world, true);
    Console.WriteLine($"Imported {world.Villages.Count} villages and {world.Villages.Sum(v => v.Cats.Count)} cats. Source was not changed."); return 0;
}
catch (Exception error) when (error is IOException || error is UnauthorizedAccessException || error is InvalidDataException || error is Newtonsoft.Json.JsonException || error is NotSupportedException || error is FormatException || error is OverflowException || error is ArgumentException)
{
    Console.Error.WriteLine($"Import refused ({error.GetType().Name}). Source and destination were preserved; inspect the compatibility requirements without copying save contents into logs."); return 1;
}
