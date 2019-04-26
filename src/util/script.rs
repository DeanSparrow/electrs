use bitcoin::blockdata::script::{Instruction::PushBytes, Script};
#[cfg(not(feature = "liquid"))]
use {
    bitcoin::network::constants::Network as BNetwork,
    bitcoin_bech32::constants::Network as B32Network,
    bitcoin_bech32::{self, u5},
    bitcoin_hashes::{hash160::Hash as Hash160, Hash},
};

use crate::chain::{address, Network};
use crate::chain::{TxIn, TxOut};

pub struct InnerScripts {
    pub redeem_script: Option<Script>,
    pub witness_script: Option<Script>,
}

pub fn script_to_address(script: &Script, network: &Network) -> Option<String> {
    // rust-elements provides an Address::from_script() utility that's not yet
    // available in rust-bitcoin, but should be soon
    #[cfg(feature = "liquid")]
    return address::Address::from_script(script, None, network.address_params())
        .map(|a| a.to_string());

    #[cfg(not(feature = "liquid"))]
    {
        let payload = if script.is_p2pkh() {
            address::Payload::PubkeyHash(Hash160::from_slice(&script[3..23]).ok()?)
        } else if script.is_p2sh() {
            address::Payload::ScriptHash(Hash160::from_slice(&script[2..22]).ok()?)
        } else if script.is_v0_p2wpkh() || script.is_v0_p2wsh() {
            let program = if script.is_v0_p2wpkh() {
                script[2..22].to_vec()
            } else {
                script[2..34].to_vec()
            };

            address::Payload::WitnessProgram(
                bitcoin_bech32::WitnessProgram::new(
                    u5::try_from_u8(0).expect("0<32"),
                    program,
                    B32Network::from(network),
                )
                .unwrap(),
            )
        } else {
            return None;
        };

        Some(
            address::Address {
                payload,
                network: BNetwork::from(network),
            }
            .to_string(),
        )
    }
}

pub fn get_script_asm(script: &Script) -> String {
    let asm = format!("{:?}", script);
    (&asm[7..asm.len() - 1]).to_string()
}

// Returns the witnessScript in the case of p2wsh, or the redeemScript in the case of p2sh.
pub fn get_innerscripts(txin: &TxIn, prevout: &TxOut) -> InnerScripts {
    // Wrapped redeemScript for P2SH spends
    let redeem_script = if prevout.script_pubkey.is_p2sh() {
        if let Some(PushBytes(redeemscript)) = txin.script_sig.iter(true).last() {
            Some(Script::from(redeemscript.to_vec()))
        } else {
            None
        }
    } else {
        None
    };

    // Wrapped witnessScript for P2WSH or P2SH-P2WSH spends
    #[cfg(not(feature = "liquid"))]
    let witness_script = if prevout.script_pubkey.is_v0_p2wsh()
        || redeem_script.as_ref().map_or(false, |s| s.is_v0_p2wsh())
    {
        txin.witness.iter().last().cloned().map(Script::from)
    } else {
        None
    };

    // TODO: witness for elements
    #[cfg(feature = "liquid")]
    let witness_script = None;

    InnerScripts {
        redeem_script,
        witness_script,
    }
}
