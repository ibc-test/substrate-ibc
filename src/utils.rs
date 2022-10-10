use crate::Config;
use scale_info::prelude::format;
use ibc::{
	applications::transfer::{error::Error as Ics20Error, VERSION},
	core::ics24_host::identifier::{ChannelId, PortId},
	signer::Signer,
};

/// Get the latest block height of the host chain
pub fn host_height<T: Config>() -> u64 {
	let block_number = format!("{:?}", <frame_system::Pallet<T>>::block_number());
	let current_height: u64 = block_number.parse().unwrap_or_default();
	current_height
}

/// In ICS20 fungible token transfer, get the escrow address by channel ID and port ID
///
/// Parameters:
/// - `port_id`: The ID of the port corresponding to the escrow.
/// - `channel_id`: The ID of the channel corresponding to the escrow.
pub fn get_channel_escrow_address(
	port_id: &PortId,
	channel_id: &ChannelId,
) -> Result<Signer, Ics20Error> {
	let contents = format!("{}/{}", port_id, channel_id);
	let mut data = VERSION.as_bytes().to_vec();
	data.extend_from_slice(&[0]);
	data.extend_from_slice(contents.as_bytes());

	let hash = sp_io::hashing::sha2_256(&data).to_vec();
	let mut hex_string = hex::encode_upper(hash);
	hex_string.insert_str(0, "0x");
	hex_string.parse::<Signer>().map_err(Ics20Error::signer)
}
