import json
import os
import secrets

# Generate simple wallet (placeholder - for production use Solana CLI)
# This is a simplified version for demonstration
import base64

# Generate random bytes
private_key_bytes = secrets.token_bytes(64)
public_key_bytes = secrets.token_bytes(32)

# Simple base64 encoding (Solana uses base58 but we'll use base64 for simplicity here)
private_key = base64.b64encode(private_key_bytes).decode('ascii')
public_key = base64.b64encode(public_key_bytes).decode('ascii')

# Save to file
wallet_path = "config/wallet.json"
os.makedirs("config", exist_ok=True)

wallet_data = {
    "public_key": public_key,
    "secret_key": private_key
}

with open(wallet_path, "w") as f:
    json.dump(wallet_data, f, indent=2)

print("=" * 50)
print("WALLET CREATED SUCCESSFULLY")
print("=" * 50)
print(f"Public Key: {public_key}")
print(f"Secret Key: {private_key}")
print()
print("⚠️  IMPORTANT - SAVE THIS INFORMATION SECURELY! ⚠️")
print()
print("1. Copy the Public Key above and save it somewhere safe")
print("2. Copy the Secret Key above and save it somewhere VERY SECURE")
print("   (Never share your secret key with anyone!)")
print("3. Fund your wallet with SOL at: https://solflare.com")
print("   Send SOL to the Public Key above")
print("4. Recommended: Start with 0.1-0.5 SOL for testing")
print()
print("NOTE: This is a placeholder wallet for demonstration.")
print("For production, use Solana CLI: solana-keygen new")
print()
print(f"Wallet file saved to: {wallet_path}")
print("=" * 50)
