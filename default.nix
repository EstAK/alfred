{
  lib,
  rustPlatform,
  src,
}: 

rustPlatform.buildRustPackage rec {
  pname = "alfred";
  version = "0.1.0";

  # Use builtins.fetchGit instead of fetchFromGitHub
  inherit src;

  cargoHash = "sha256-2qtBcq1B7cZeCtJ/ag0Dj63QalmPmgE0VZdKnbgROyU=";

  meta = with lib; {
    description = "A cli for selecting and initializing nix templates";
    license = licenses.gpl3Plus;
  };
}
