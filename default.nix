{
  lib,
  rustPlatform,
}: 

rustPlatform.buildRustPackage rec {
  pname = "alfred";
  version = "0.1.0";

  # Use builtins.fetchGit instead of fetchFromGitHub
  src = builtins.fetchGit {
    url = "git@github.com:estak/alfred.git";
    rev = "d4d44297f98d37382d61f918583f627e513b646c"; 
  };

  cargoHash = "sha256-2qtBcq1B7cZeCtJ/ag0Dj63QalmPmgE0VZdKnbgROyU=";

  meta = with lib; {
    description = "A cli for selecting and initializing nix templates";
    license = licenses.gpl3Plus;
  };
}
