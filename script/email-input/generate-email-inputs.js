const { verifyDKIMSignature } = require("@zk-email/helpers/dist/dkim/");
const fs = require("fs");
const path = require("path");

function bigintToBase64(bigintValue) {
    // Convert BigInt to hex string, remove '0x' if present
    const hexString = bigintValue.toString(16).replace("0x", "");
    // Pad with 0 if odd length
    const paddedHex = hexString.length % 2 ? "0" + hexString : hexString;
    return Buffer.from(paddedHex, "hex").toString("base64");
}

// Parse raw email to circuit inputs and save that to ./email-inputs.json to be used by the main rust program.
async function generateEmailInputs() {
    const rawEmail = fs.readFileSync(
        path.join(__dirname, "./email.eml"),
        "utf8"
    );
    const dkimResult = await verifyDKIMSignature(rawEmail);
    fs.writeFileSync(
        "./email-inputs.json",
        JSON.stringify({
            signature: bigintToBase64(dkimResult.signature),
            publicKey: bigintToBase64(dkimResult.publicKey),
            headers: dkimResult.headers.toString(),
            body: dkimResult.body.toString(),
            bodyHash: dkimResult.bodyHash.toString(),
        })
    );
}

generateEmailInputs()
    .then(() => {})
    .catch((error) => {
        console.error("Unhandled error:", error);
    });
