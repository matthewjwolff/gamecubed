use evdev::uinput::VirtualDeviceBuilder;
use evdev::{AttributeSet, EventType, InputEvent, Key};
use libusb::Context;
use std::time::Duration;
use std::thread::sleep;

fn main() {
    // somehow it knows this is a joystick (and not a keyboard or mouse or something)
    let mut attr_set = AttributeSet::<Key>::new();
    attr_set.insert(Key::BTN_SOUTH);
    let mut device = VirtualDeviceBuilder::new()
        .unwrap()
        .name("Gamecube Controller Port 1")
        .with_keys(&attr_set)
        .unwrap()
        .build()
        .unwrap();

    loop {
        device.emit(&[InputEvent::new(EventType::KEY, 304, 1)]).expect("Erorr pushing event"); // pushes BTN_SOUTH down, need to find documentation for these
        sleep(Duration::from_secs(1));
        device.emit(&[InputEvent::new(EventType::KEY, 304, 0)]).expect("Erorr pushing event"); // pushes BTN_SOUTH down, need to find documentation for these
        sleep(Duration::from_secs(1));
    }
    
}

fn main2() {
    // get all usb devices

    // find the gc adapter
    let cx = Context::new().expect("Error loading libUSB");
    let devices = cx.devices().expect("Could not get devices");
    for device in devices.iter() {
        // TODO find the gamecube adapter
        let descriptor = device
            .device_descriptor()
            .expect("Could not get device descriptor");

        
        if descriptor.vendor_id() == 0x057e && descriptor.product_id() == 0x0337 {
            // this is the device
            let mut dev_handle = device.open().expect("Could not open device");
            dev_handle.reset().expect("Could not reset device");
            if dev_handle.kernel_driver_active(0).expect("Could not query kernel driver status") {
                dev_handle.detach_kernel_driver(0).expect("Could not detatch kernel driver");
            }
            dev_handle.claim_interface(0).expect("Could not claim interface");

            // figure out the configuration
            let mut endpoint_in = 0;
            let mut _endpoint_out = 0;
            // this is what dolphin does, i guess this handles the embedded usb port? you would think they could hardcode it
            let config = device.config_descriptor(0).expect("Could not get active config descriptor");
            for interface in config.interfaces() {  
                for interface_descriptor in interface.descriptors() {
                    for endpoint_descriptor in interface_descriptor.endpoint_descriptors() {
                        if (endpoint_descriptor.address() & 0x80)!=0 { // TODO this is LIBUSB_ENDPOINT_IN, use the rust library instead
                            // in 
                            endpoint_in = endpoint_descriptor.address();
                        } else {
                            // out
                            _endpoint_out = endpoint_descriptor.address();
                        }
                    }
                }
            }
            

            
            //dev.reset().expect("Could not reset device");
            let timeout = Duration::from_millis(1000); // from dolphin source

            

            // now we have to write this magic number?
            // stalls forever here?
            // TODO apparently don't have to do this, even though dolphin does?
            //let write_result = dev_handle.write_interrupt(0x02, &[0x13], Duration::from_millis(0));
            //println!("Wrote {} bytes", write_result.expect("could not write to device"));

            // now do whatever dolphin does...
            const PAYLOAD_SIZE: usize = 37; // from dolphin source
            

            let mut buffer: [u8; 37] = [0; PAYLOAD_SIZE];
            loop {
                let result = dev_handle.read_interrupt(endpoint_in, &mut buffer, timeout);
                match result {
                    Ok(size) => {
                        print!("{} ", size); // this semicolon is necessary?
                        for i in buffer  {
                            print!("{} ", i)
                        }
                        println!();
                    },
                    Err(error) => println!("ERROR {}", error),
                }
            }
        }
    }
    // TODO safe reset on terminate?
}
