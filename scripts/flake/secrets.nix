{ root, lib, ... }:

let
  pidgeons =
    builtins.map
      (pidgeon: pidgeon // {
        wifi =
          if pidgeon ? wifi
          then pidgeon.wifi
          else pidgeon.id;
      })
      (builtins.fromJSON
        (builtins.readFile
          (lib.path.append
            root
            "assets/pidgeon/pidgeons.json")));

  secrets =
    builtins.listToAttrs
      (builtins.map
        (pidgeon:
          let
            secrets = {
              sopsFile = lib.path.append root "assets/secrets/${pidgeon.id}.yaml";
              ageKeyFile = "/root/host.scrt.key";
            };

            secrets.keys = {
              postgresSslKeyFile = "postgres.crt";
              postgresSslCertFile = "postgres.crt.pub";
              postgresInitialScript = "postgres.sql";
              networkManagerEnvironmentFile = "wifi.env";
              nebulaKey = "nebula.crt";
              nebulaCert = "nebula.crt.pub";
              nebulaCa = "nebula.ca.pub";
              userHashedPasswordFile = "altibiz.pass.pub";
              userAuthorizedKeys = "altibiz.ssh.pub";
              pidgeonEnv = "pidgeon.env";
            };
          in
          {
            name = "pidgeon-${pidgeon.id}-raspberryPi4-aarch64-linux";
            value = secrets;
          })
        pidgeons);

  rumor =
    builtins.listToAttrs
      (builtins.map
        (pidgeon:
          let
            name = "pidgeon-${pidgeon.id}-raspberryPi4-aarch64-linux";

            instanceSecrets = secrets.${name};

            files = {
              # shared
              postgresCaPrivate = "postgres-ca-priv";
              postgresCaPublic = "postgres-ca-pub";
              postgresCaSerial = "postgres-ca-srl";
              nebulaCaPrivate = "nebula-ca-priv";
              nebulaCaPublic = "nebula-ca-pub";

              # instance
              postgresSslPrivate = "db.key";
              postgresSslPublic = "db.key.pub";
              postgresPidgeonPassword = "pidgeon.db.user";
              postgresUserPassword = "altibiz.db.user";
              postgresPassword = "postgres.db.user";
              postgresSql = "db.sql";
              nebulaSslPrivate = "vpn.key";
              nebulaSslPublic = "vpn.key.pub";
              userPasswordPrivate = "pass";
              userPasswordPublic = "pass.pub";
              userSshPrivate = "ssh.key";
              userSshPublic = "ssh.key.pub";
              wifiAdmin = "wifi.admin";
              wifiWps = "wifi.wps";
              wifiSsid = "wifi.ssid.pub";
              wifiSsidSuffix = "wifi.ssid.suffix";
              wifiPassword = "wifi.pass";
              wifiEnv = "wifi.env";
              pidgeonApiKey = "key";
              pidgeonEnv = "pidgeon.env";
              agePublic = "scrt.key.pub";
              agePrivate = "scrt.key";
              secretsPublic = "scrt.val.pub";
              secretsPrivate = "scrt.val";
            };

            rumor.imports = [
              {
                importer = "vault";
                arguments.path = "kv/ozds/pidgeon/${pidgeon.id}";
                arguments.allow_fail = true;
              }
              {
                importer = "vault-file";
                arguments.path = "kv/ozds/shared";
                arguments.file = files.postgresCaPrivate;
              }
              {
                importer = "vault-file";
                arguments.path = "kv/ozds/shared";
                arguments.file = files.postgresCaPublic;
              }
              {
                importer = "vault-file";
                arguments.path = "kv/ozds/shared";
                arguments.file = files.postgresCaSerial;
              }
              {
                importer = "vault-file";
                arguments.path = "kv/ozds/shared";
                arguments.file = files.nebulaCaPrivate;
              }
              {
                importer = "vault-file";
                arguments.path = "kv/ozds/shared";
                arguments.file = files.nebulaCaPublic;
              }
              {
                importer = "vault-file";
                arguments.path = "kv/ozds/pidgeon/${pidgeon.wifi}";
                arguments.file = files.wifiSsid;
              }
              {
                importer = "vault-file";
                arguments.path = "kv/ozds/pidgeon/${pidgeon.wifi}";
                arguments.file = files.wifiPassword;
              }
            ];

            rumor.exports = [
              {
                exporter = "vault-file";
                arguments.path = "kv/ozds/shared";
                arguments.file = files.postgresCaSerial;
              }
              {
                exporter = "vault";
                arguments.path = "kv/ozds/pidgeon/${pidgeon.id}";
              }
              {
                exporter = "copy";
                arguments.from = files.secretsPublic;
                arguments.to = builtins.toString instanceSecrets.sopsFile;
              }
            ];

            rumor.generations = [
              {
                generator = "openssl";
                arguments = {
                  ca_private = files.postgresCaPrivate;
                  ca_public = files.postgresCaPublic;
                  serial = files.postgresCaSerial;
                  name = pidgeon.ip;
                  private = files.postgresSslPrivate;
                  public = files.postgresSslPublic;
                };
              }
              {
                generator = "key";
                arguments = {
                  name = files.postgresPidgeonPassword;
                  length = 32;
                };
              }
              {
                generator = "key";
                arguments = {
                  name = files.postgresUserPassword;
                  length = 32;
                };
              }
              {
                generator = "key";
                arguments = {
                  name = files.postgresPassword;
                  length = 32;
                };
              }
              {
                generator = "moustache";
                arguments = {
                  name = files.postgresSql;
                  variables = {
                    POSTGRES_PASS = files.postgresPassword;
                    PIDGEON_POSTGRES_PASS = files.postgresPidgeonPassword;
                    USER_POSTGRES_PASS = files.postgresUserPassword;
                  };
                  template = ''
                    ALTER USER postgres WITH PASSWORD '{{POSTGRES_PASS}}';

                    CREATE USER pidgeon PASSWORD '{{PIDGEON_POSTGRES_PASS}}';
                    CREATE USER altibiz PASSWORD '{{USER_POSTGRES_PASS}}';

                    CREATE DATABASE pidgeon;
                    ALTER DATABASE pidgeon OWNER TO pidgeon;

                    \c pidgeon

                    GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO altibiz;
                    GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO altibiz;
                    GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public TO altibiz;
                  '';
                  renew = true;
                };
              }
              {
                generator = "key";
                arguments = {
                  name = files.wifiAdmin;
                  length = 7;
                };
              }
              {
                generator = "key";
                arguments = {
                  name = files.wifiPassword;
                  length = 32;
                };
              }
              {
                generator = "key";
                arguments = {
                  name = files.wifiSsidSuffix;
                  length = 16;
                };
              }
              {
                generator = "moustache";
                arguments = {
                  name = files.wifiSsid;
                  variables = {
                    WIFI_SSID_SUFFIX = files.wifiSsidSuffix;
                  };
                  template = ''pidgeon-{{WIFI_SSID_SUFFIX}}'';
                  renew = true;
                };
              }
              {
                generator = "pin";
                arguments = {
                  name = files.wifiWps;
                  length = 4;
                };
              }
              {
                generator = "env";
                arguments = {
                  name = files.wifiEnv;
                  variables = {
                    WIFI_SSID = files.wifiSsid;
                    WIFI_PASS = files.wifiPassword;
                  };
                  renew = true;
                };
              }
              {
                generator = "nebula";
                arguments = {
                  ca_private = files.nebulaCaPrivate;
                  ca_public = files.nebulaCaPublic;
                  name = pidgeon.ip;
                  ip = "${pidgeon.ip}/16";
                  private = files.nebulaSslPrivate;
                  public = files.nebulaSslPublic;
                };
              }
              {
                generator = "mkpasswd";
                arguments = {
                  public = files.userPasswordPublic;
                  private = files.userPasswordPrivate;
                };
              }
              {
                generator = "ssh-keygen";
                arguments = {
                  name = pidgeon.ip;
                  public = files.userSshPublic;
                  private = files.userSshPrivate;
                };
              }
              {
                generator = "key";
                arguments = {
                  name = files.pidgeonApiKey;
                  length = 32;
                };
              }
              {
                generator = "env";
                arguments = {
                  name = files.pidgeonEnv;
                  variables = {
                    PIDGEON_DB_DOMAIN = "localhost";
                    PIDGEON_DB_PORT = "5433";
                    PIDGEON_DB_USER = "pidgeon";
                    PIDGEON_DB_PASSWORD = files.postgresPidgeonPassword;
                    PIDGEON_DB_NAME = "pidgeon";

                    PIDGEON_CLOUD_DOMAIN = "ozds.altibiz.com";
                    PIDGEON_CLOUD_API_KEY = files.pidgeonApiKey;
                    PIDGEON_CLOUD_ID = "pidgeon-${pidgeon.id}";

                    PIDGEON_NETWORK_IP_RANGE_START = "127.0.0.1";
                    PIDGEON_NETWORK_IP_RANGE_END = "127.0.0.1";
                  };
                  renew = true;
                };
              }
              {
                generator = "age";
                arguments = {
                  private = files.agePrivate;
                  public = files.agePublic;
                };
              }
              {
                generator = "sops";
                arguments = {
                  age = files.agePublic;
                  private = files.secretsPrivate;
                  public = files.secretsPublic;
                  secrets = {
                    ${instanceSecrets.keys.postgresSslKeyFile} = files.postgresSslPrivate;
                    ${instanceSecrets.keys.postgresSslCertFile} = files.postgresSslPublic;
                    ${instanceSecrets.keys.postgresInitialScript} = files.postgresSql;
                    ${instanceSecrets.keys.networkManagerEnvironmentFile} = files.wifiEnv;
                    ${instanceSecrets.keys.nebulaKey} = files.nebulaSslPrivate;
                    ${instanceSecrets.keys.nebulaCert} = files.nebulaSslPublic;
                    ${instanceSecrets.keys.nebulaCa} = files.nebulaCaPublic;
                    ${instanceSecrets.keys.userHashedPasswordFile} = files.userPasswordPublic;
                    ${instanceSecrets.keys.userAuthorizedKeys} = files.userSshPublic;
                    ${instanceSecrets.keys.pidgeonEnv} = files.pidgeonEnv;
                  };
                  renew = true;
                };
              }
            ];
          in
          {
            inherit name;
            value = rumor;
          })
        pidgeons);
in
{
  flake.lib.pidgeons = pidgeons;
  flake.lib.secrets = secrets;
  flake.lib.rumor = rumor;
}
