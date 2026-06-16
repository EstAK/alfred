{
  lib,
  rustPlatform,
}: 

rustPlatform.buildRustPackage rec {
  pname = "alfred";
  version = "0.1.0";

  # Use builtins.fetchGit instead of fetchFromGitHub
  src = builtins.fetchGit {
    url = "https://github.com/EstAK/alfred.git";
    rev = "47e4289101376c386b54fbdb0691c73799849816"; 
  };

  cargoHash = "sha256-2qtBcq1B7cZeCtJ/ag0Dj63QalmPmgE0VZdKnbgROyU=";

  meta = with lib; {
    description = "A cli for selecting and initializing nix templates";
    license = licenses.gpl3Plus;
  };
}
