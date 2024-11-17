let
  nixpkgs = fetchTarball "https://github.com/NixOS/nixpkgs/tarball/nixos-24.05";
  pkgs = import nixpkgs { config = {}; overlays = []; };
in

pkgs.mkShellNoCC {
  packages = with pkgs; [
    postgresql
  ];

  shellHook = ''
    # Check if a environment variables has been set, otherwise use default values
    export PGDATA="''${PGDATA:-$HOME/Project/database/pgsql}"
    export PGHOST="''${PGHOST:-localhost}"
    export PGDATABASE="''${PGDATABASE:-postgres}"
    export PGPORT="''${PGPORT:-5432}"

    # If the data directory does not exist, initialize the database.
    if [ ! -d $PGDATA ]; then
      echo 'Initializing postgresql database...'
      initdb $PGDATA --auth=trust >/dev/null
    fi

    # Start PostgreSQL
    pg_ctl start -l "/tmp/postgres.log"

    # When exiting the shell, stop the PG service.
    cleanup() {
      pg_ctl stop
    }
    trap cleanup EXIT
  '';
}
