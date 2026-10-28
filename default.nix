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

  cargoHash = "sha256-XxkagySCtjGlVWm1pt+4NXefSSyMpXbinGrUA8Kbdrw=";

  meta = with lib; {
    description = "A cli for selecting and initializing nix templates";
    license = licenses.gpl3Plus;
  };
}
