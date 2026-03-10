const {
  createWriteStream,
  existsSync,
  mkdirSync,
  mkdtemp,
  unlinkSync,
  rmdirSync,
} = require("fs");
const { join, sep } = require("path");
const { spawnSync } = require("child_process");
const { tmpdir } = require("os");
const { createHash, timingSafeEqual } = require("crypto");

const axios = require("axios");
const rimraf = require("rimraf");
const tmpDir = tmpdir();

const error = (msg) => {
  console.error(msg);
  process.exit(1);
};

/**
 * Fetch the expected SHA256 checksum from the release's .sha256 sidecar file.
 * Returns the hex digest string, or null if the checksum file is unavailable.
 */
async function fetchExpectedChecksum(artifactUrl, fetchOptions) {
  const checksumUrl = `${artifactUrl}.sha256`;
  try {
    const res = await axios({
      ...fetchOptions,
      url: checksumUrl,
      responseType: "text",
    });
    // Format: "<hex> *<filename>" or "<hex>  <filename>"
    const line = (res.data || "").trim();
    const hash = line.split(/\s+/)[0];
    if (hash && /^[0-9a-f]{64}$/i.test(hash)) {
      return hash.toLowerCase();
    }
    console.warn(
      `Warning: SHA256 checksum file has unexpected format: ${checksumUrl}`,
    );
    return null;
  } catch (e) {
    console.warn(
      `Warning: Could not fetch SHA256 checksum (${e.message}). Skipping verification.`,
    );
    return null;
  }
}

class Package {
  constructor(platform, name, url, filename, zipExt, binaries) {
    let errors = [];
    if (typeof url !== "string") {
      errors.push("url must be a string");
    } else {
      try {
        new URL(url);
      } catch (e) {
        errors.push(e);
      }
    }
    if (name && typeof name !== "string") {
      errors.push("package name must be a string");
    }
    if (!name) {
      errors.push("You must specify the name of your package");
    }
    if (binaries && typeof binaries !== "object") {
      errors.push("binaries must be a string => string map");
    }
    if (!binaries) {
      errors.push("You must specify the binaries in the package");
    }

    if (errors.length > 0) {
      let errorMsg =
        "One or more of the parameters you passed to the Binary constructor are invalid:\n";
      errors.forEach((error) => {
        errorMsg += error;
      });
      errorMsg +=
        '\n\nCorrect usage: new Package("my-binary", "https://example.com/binary/download.tar.gz", {"my-binary": "my-binary"})';
      error(errorMsg);
    }

    this.platform = platform;
    this.url = url;
    this.name = name;
    this.filename = filename;
    this.zipExt = zipExt;
    this.installDirectory = join(__dirname, "node_modules", ".bin_real");
    this.binaries = binaries;

    if (!existsSync(this.installDirectory)) {
      mkdirSync(this.installDirectory, { recursive: true });
    }
  }

  exists() {
    for (const binaryName in this.binaries) {
      const binRelPath = this.binaries[binaryName];
      const binPath = join(this.installDirectory, binRelPath);
      if (!existsSync(binPath)) {
        return false;
      }
    }
    return true;
  }

  install(fetchOptions, suppressLogs = false) {
    if (this.exists()) {
      if (!suppressLogs) {
        console.error(
          `${this.name} is already installed, skipping installation.`,
        );
      }
      return Promise.resolve();
    }

    if (existsSync(this.installDirectory)) {
      rimraf.sync(this.installDirectory);
    }

    mkdirSync(this.installDirectory, { recursive: true });

    if (!suppressLogs) {
      console.error(`Downloading release from ${this.url}`);
    }

    // Checksum verification is best-effort. If the .sha256 sidecar file
    // is unavailable (e.g., older releases), installation proceeds without
    // verification. Set REQUIRE_CHECKSUM=1 to make verification mandatory.
    return fetchExpectedChecksum(this.url, fetchOptions)
      .then((expectedChecksum) => {
        return axios({
          ...fetchOptions,
          url: this.url,
          responseType: "stream",
        }).then((res) => {
          return new Promise((resolve, reject) => {
            mkdtemp(`${tmpDir}${sep}`, (err, directory) => {
              if (err) return reject(err);

              let tempFile = join(directory, this.filename);
              const hash = createHash("sha256");
              const sink = createWriteStream(tempFile);

              res.data.on("data", (chunk) => {
                hash.update(chunk);
              });
              res.data.on("error", (err) => reject(err));

              res.data.pipe(sink);

              sink.on("error", (err) => reject(err));
              sink.on("close", () => {
                const computedHash = hash.digest("hex");

                if (expectedChecksum) {
                  const computedBuf = Buffer.from(computedHash, "hex");
                  const expectedBuf = Buffer.from(expectedChecksum, "hex");
                  if (!timingSafeEqual(computedBuf, expectedBuf)) {
                    try { unlinkSync(tempFile); } catch (_) {}
                    try { rmdirSync(directory); } catch (_) {}
                    reject(
                      new Error(
                        `SHA256 checksum mismatch for ${this.url}\n` +
                          `  Expected: ${expectedChecksum}\n` +
                          `  Got:      ${computedHash}\n` +
                          `This may indicate a corrupted or tampered download. Aborting.`,
                      ),
                    );
                    return;
                  }
                  if (!suppressLogs) {
                    console.error(`SHA256 checksum verified: ${computedHash}`);
                  }
                }

                if (/\.tar\.*/.test(this.zipExt)) {
                  const result = spawnSync("tar", [
                    "xf",
                    tempFile,
                    "--strip-components",
                    "1",
                    "-C",
                    this.installDirectory,
                  ]);
                  if (result.status == 0) {
                    resolve();
                  } else if (result.error) {
                    reject(result.error);
                  } else {
                    reject(
                      new Error(
                        `An error occurred untarring the artifact: stdout: ${result.stdout}; stderr: ${result.stderr}`,
                      ),
                    );
                  }
                } else if (this.zipExt == ".zip") {
                  let result;
                  if (this.platform.artifactName.includes("windows")) {
                    result = spawnSync("powershell.exe", [
                      "-NoProfile",
                      "-NonInteractive",
                      "-Command",
                      `& {
                        param([string]$LiteralPath, [string]$DestinationPath)
                        Expand-Archive -LiteralPath $LiteralPath -DestinationPath $DestinationPath -Force
                    }`,
                      tempFile,
                      this.installDirectory,
                    ]);
                  } else {
                    result = spawnSync("unzip", [
                      "-q",
                      tempFile,
                      "-d",
                      this.installDirectory,
                    ]);
                  }

                  if (result.status == 0) {
                    resolve();
                  } else if (result.error) {
                    reject(result.error);
                  } else {
                    reject(
                      new Error(
                        `An error occurred unzipping the artifact: stdout: ${result.stdout}; stderr: ${result.stderr}`,
                      ),
                    );
                  }
                } else {
                  reject(
                    new Error(`Unrecognized file extension: ${this.zipExt}`),
                  );
                }
              });
            });
          });
        });
      })
      .then(() => {
        if (!suppressLogs) {
          console.error(`${this.name} has been installed!`);
        }
      })
      .catch((e) => {
        error(`Error fetching release: ${e.message}`);
      });
  }

  run(binaryName, fetchOptions) {
    const promise = !this.exists()
      ? this.install(fetchOptions, true)
      : Promise.resolve();

    promise
      .then(() => {
        const [, , ...args] = process.argv;

        const options = { cwd: process.cwd(), stdio: "inherit" };

        const binRelPath = this.binaries[binaryName];
        if (!binRelPath) {
          error(`${binaryName} is not a known binary in ${this.name}`);
        }
        const binPath = join(this.installDirectory, binRelPath);
        const result = spawnSync(binPath, args, options);

        if (result.error) {
          error(result.error);
        }

        process.exit(result.status);
      })
      .catch((e) => {
        error(e.message);
        process.exit(1);
      });
  }
}

module.exports.Package = Package;
