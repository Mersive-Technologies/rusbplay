# rusbplay

Given hard-coded device settings, play a sin wave directly to a UAC device trough
user-land libusb1-sys.

## Setup

```shell
git clone
cargo run
```

### Device descriptor

```text
Root
	Config(DescriptorConfig { w_total_length: 378, b_num_interfaces: 4, b_configuration_value: 1, i_configuration: 4, bm_attributes: 160, b_max_power: 50 })
		Interface(DescriptorInterface { b_interface_number: 0, b_alternate_setting: 0, b_num_endpoints: 1, b_interface_class: 1, b_interface_sub_class: 1, b_interface_protocol: 0, i_interface: 5 })
			UacAcHeader(Uac1AcHeaderDescriptor { bcd_adc: 256, w_total_length: 93, b_in_collection: 2, ba_interface_nr: [1, 2] })
			UacInputTerminal(UacInputTerminalDescriptor { b_terminal_id: 1, w_terminal_type: 513, b_assoc_terminal: 0, b_nr_channels: 2, w_channel_config: 3, i_channel_names: 0, i_terminal: 0 })
			UacFeatureUnit(UacFeatureUnitDescriptor { b_unit_id: 3, b_source_id: 1, b_control_size: 2, bma_controls: [1, 0, 2, 0, 2, 0, 0] })
			UacOutputTerminal(Uac1OutputTerminalDescriptor { b_terminal_id: 2, w_terminal_type: 257, b_assoc_terminal: 1, b_source_id: 4, i_terminal: 0 })
			DescriptorUacInterfaceUnknown(DescriptorUacInterfaceUnknown { iface_subclass: 8, bytes: [4, 218, 11, 1, 3, 2, 3, 0, 0, 1, 1, 0] })
			UacInputTerminal(UacInputTerminalDescriptor { b_terminal_id: 14, w_terminal_type: 257, b_assoc_terminal: 0, b_nr_channels: 2, w_channel_config: 3, i_channel_names: 0, i_terminal: 21 })
			UacOutputTerminal(Uac1OutputTerminalDescriptor { b_terminal_id: 15, w_terminal_type: 770, b_assoc_terminal: 14, b_source_id: 16, i_terminal: 0 })
			UacFeatureUnit(UacFeatureUnitDescriptor { b_unit_id: 16, b_source_id: 14, b_control_size: 2, bma_controls: [1, 0, 2, 0, 2, 0, 0] })
			UacEndpoint(UacDescriptorEndpoint { b_endpoint_address: 135, bm_attributes: 3, w_max_packet_size: 16, b_interval: 8, b_refresh: 0, b_synch_address: 0 })
		Interface(DescriptorInterface { b_interface_number: 1, b_alternate_setting: 0, b_num_endpoints: 0, b_interface_class: 1, b_interface_sub_class: 2, b_interface_protocol: 0, i_interface: 18 })
			Interface(DescriptorInterface { b_interface_number: 1, b_alternate_setting: 1, b_num_endpoints: 1, b_interface_class: 1, b_interface_sub_class: 2, b_interface_protocol: 0, i_interface: 0 })
				UacAsGeneral(Uac1AsHeaderDescriptor { b_terminal_link: 2, b_delay: 1, w_format_tag: 1 })
				UacFormatTypeI(UacFormatTypeIContinuousDescriptor { b_nr_channels: 2, b_subframe_size: 2, b_bit_resolution: 16, b_sam_freq_type: 1, t_sam_freq: [44100] })
				UacEndpoint(UacDescriptorEndpoint { b_endpoint_address: 129, bm_attributes: 5, w_max_packet_size: 228, b_interval: 4, b_refresh: 0, b_synch_address: 0 })
				UacIsoEndpointDescriptor(UacIsoEndpointDescriptor { b_descriptor_subtype: 1, bm_attributes: 1, b_lock_delay_units: 0, w_lock_delay: 0 })
			Interface(DescriptorInterface { b_interface_number: 1, b_alternate_setting: 2, b_num_endpoints: 1, b_interface_class: 1, b_interface_sub_class: 2, b_interface_protocol: 0, i_interface: 0 })
				UacAsGeneral(Uac1AsHeaderDescriptor { b_terminal_link: 2, b_delay: 1, w_format_tag: 1 })
				UacFormatTypeI(UacFormatTypeIContinuousDescriptor { b_nr_channels: 2, b_subframe_size: 2, b_bit_resolution: 16, b_sam_freq_type: 1, t_sam_freq: [48000] })
				UacEndpoint(UacDescriptorEndpoint { b_endpoint_address: 129, bm_attributes: 5, w_max_packet_size: 248, b_interval: 4, b_refresh: 0, b_synch_address: 0 })
				UacIsoEndpointDescriptor(UacIsoEndpointDescriptor { b_descriptor_subtype: 1, bm_attributes: 1, b_lock_delay_units: 0, w_lock_delay: 0 })
		Interface(DescriptorInterface { b_interface_number: 2, b_alternate_setting: 0, b_num_endpoints: 0, b_interface_class: 1, b_interface_sub_class: 2, b_interface_protocol: 0, i_interface: 21 })
			Interface(DescriptorInterface { b_interface_number: 2, b_alternate_setting: 1, b_num_endpoints: 1, b_interface_class: 1, b_interface_sub_class: 2, b_interface_protocol: 0, i_interface: 0 })
				UacAsGeneral(Uac1AsHeaderDescriptor { b_terminal_link: 14, b_delay: 1, w_format_tag: 1 })
				UacFormatTypeI(UacFormatTypeIContinuousDescriptor { b_nr_channels: 2, b_subframe_size: 2, b_bit_resolution: 16, b_sam_freq_type: 1, t_sam_freq: [44100] })
				UacEndpoint(UacDescriptorEndpoint { b_endpoint_address: 4, bm_attributes: 9, w_max_packet_size: 228, b_interval: 4, b_refresh: 0, b_synch_address: 0 })
				UacIsoEndpointDescriptor(UacIsoEndpointDescriptor { b_descriptor_subtype: 1, bm_attributes: 1, b_lock_delay_units: 0, w_lock_delay: 0 })
			Interface(DescriptorInterface { b_interface_number: 2, b_alternate_setting: 2, b_num_endpoints: 1, b_interface_class: 1, b_interface_sub_class: 2, b_interface_protocol: 0, i_interface: 0 })
				UacAsGeneral(Uac1AsHeaderDescriptor { b_terminal_link: 14, b_delay: 1, w_format_tag: 1 })
				UacFormatTypeI(UacFormatTypeIContinuousDescriptor { b_nr_channels: 2, b_subframe_size: 2, b_bit_resolution: 16, b_sam_freq_type: 1, t_sam_freq: [48000] })
				UacEndpoint(UacDescriptorEndpoint { b_endpoint_address: 4, bm_attributes: 9, w_max_packet_size: 248, b_interval: 4, b_refresh: 0, b_synch_address: 0 })
				UacIsoEndpointDescriptor(UacIsoEndpointDescriptor { b_descriptor_subtype: 1, bm_attributes: 1, b_lock_delay_units: 0, w_lock_delay: 0 })
			Interface(DescriptorInterface { b_interface_number: 2, b_alternate_setting: 3, b_num_endpoints: 1, b_interface_class: 1, b_interface_sub_class: 2, b_interface_protocol: 0, i_interface: 0 })
				UacAsGeneral(Uac1AsHeaderDescriptor { b_terminal_link: 14, b_delay: 1, w_format_tag: 1 })
				UacFormatTypeI(UacFormatTypeIContinuousDescriptor { b_nr_channels: 2, b_subframe_size: 2, b_bit_resolution: 16, b_sam_freq_type: 1, t_sam_freq: [96000] })
				UacEndpoint(UacDescriptorEndpoint { b_endpoint_address: 4, bm_attributes: 9, w_max_packet_size: 496, b_interval: 4, b_refresh: 0, b_synch_address: 0 })
				UacIsoEndpointDescriptor(UacIsoEndpointDescriptor { b_descriptor_subtype: 1, bm_attributes: 1, b_lock_delay_units: 0, w_lock_delay: 0 })
		Interface(DescriptorInterface { b_interface_number: 3, b_alternate_setting: 0, b_num_endpoints: 1, b_interface_class: 3, b_interface_sub_class: 0, b_interface_protocol: 0, i_interface: 0 })
			CsDevice(DescriptorCsDevice { bytes: [17, 1, 0, 1, 34, 57, 0] })
			Endpoint(DescriptorEndpoint { b_endpoint_address: 136, bm_attributes: 3, w_max_packet_size: 16, b_interval: 4 })
```

#### How to interpret
                              
```rust
const LIBUSB_ENDPOINT_IN: u8 = 0x80; // if an endpoint is >= 128, it's input
```

So we have two endpoints:
1. a mic
2. a speaker

