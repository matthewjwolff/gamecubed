use evdev::uinput::{VirtualDevice, VirtualDeviceBuilder};
use evdev::{AttributeSet, EventType, InputEvent, Key};
use libusb::Context;
use std::time::Duration;
use std::thread::sleep;

fn main2() {
    // somehow it knows this is a joystick (and not a keyboard or mouse or something)
    let attr_set = {
        // keep the mutable ref inside this scope
        let mut attr_set = AttributeSet::<Key>::new();
        // four face buttons
        attr_set.insert(Key::BTN_NORTH);
        attr_set.insert(Key::BTN_SOUTH);
        attr_set.insert(Key::BTN_WEST);
        attr_set.insert(Key::BTN_EAST);

        // dpad
        attr_set.insert(Key::BTN_DPAD_DOWN);
        attr_set.insert(Key::BTN_DPAD_LEFT);
        attr_set.insert(Key::BTN_DPAD_RIGHT);
        attr_set.insert(Key::BTN_DPAD_UP);

        // TODO this will probably need to be configured at runtime
        // Z mapped to select (weird)
        attr_set.insert(Key::BTN_SELECT);
        // L click mapped to LB
        attr_set.insert(Key::BTN_TR);
        // R click mapped ot RB
        attr_set.insert(Key::BTN_TL);
        // start is fine
        attr_set.insert(Key::BTN_START);

        attr_set
    };
    
    let mut device = VirtualDeviceBuilder::new()
        .unwrap()
        .name("Gamecube Controller Port 1")
        .with_keys(&attr_set)
        .unwrap()
        .build()
        .unwrap();

        // apparently the kernel will ignore repeated down events without an up event (it knows you can't push a key that's already been pushed?)
    loop {
        // note that events that are expected to be simultaneous should be part of the same array
        device.emit(&[InputEvent::new(EventType::KEY, 304, 1)]).expect("Erorr pushing event"); // pushes BTN_SOUTH down, need to find documentation for these
        sleep(Duration::from_secs(1));
        device.emit(&[InputEvent::new(EventType::KEY, 304, 0)]).expect("Erorr pushing event"); // pushes BTN_SOUTH down, need to find documentation for these
        sleep(Duration::from_secs(1));
    }
    
}

enum ControllerType {
    None,
    Wired,
    Wireless
}
fn main() {
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
            let mut devices: [Option<VirtualDevice>;4] = [Option::None, Option::None, Option::None, Option::None];
            loop {
                let result = dev_handle.read_interrupt(endpoint_in, &mut buffer, timeout);
                match result {
                    Ok(size) => {

                        for i in buffer {
                            print!("{} ", i);
                        }
                        println!("");
                        
                        // from dolphin source code
                        for i in 0..4 {
                            // controller_payload_size = 9;
                            let controller_payload = &buffer[i*9+1..i*9+10];
                            let controller_type_pl = controller_payload[0];
                            let controller_type = if (controller_type_pl & 0b00010000)!=0 { ControllerType::Wired} else if (controller_type_pl & 0b00100000)!=0 {ControllerType::Wireless} else {ControllerType::None};

                            // actual buttons are bits in these two
                            let buttons_1 = controller_payload[1];
                            let a = (buttons_1 & 0b00000001) != 0;
                            let b = (buttons_1 & 0b00000010) != 0;
                            let x = (buttons_1 & 0b00000100) != 0;
                            let y = (buttons_1 & 0b00001000) != 0;

                            let dpad_left =  (buttons_1 & 0b00010000) != 0;
                            let dpad_right = (buttons_1 & 0b00100000) != 0;
                            let dpad_down =  (buttons_1 & 0b01000000) != 0;
                            let dpad_up =    (buttons_1 & 0b10000000) != 0;

                            let buttons_2 = controller_payload[2];
                            let start =   (buttons_2 & 0b00000000) != 0;
                            let z: bool =       (buttons_2 & 0b00000010) != 0;
                            let r_click = (buttons_2 & 0b00000100) != 0;
                            let l_click = (buttons_2 & 0b00001000) != 0;
                            // i guess some unused bits in byte 3?

                            let stick_x = controller_payload[3];
                            let stick_y = controller_payload[4];
                            let cstick_x = controller_payload[5];
                            let cstick_y = controller_payload[6];
                            let l = controller_payload[7];
                            let r = controller_payload[8];
                        }
                         
                        // TODO what if size not 37?

                        // from dolphin source code
                        // chan=0, index=1
                        // chan=1, index=10
                        // chan=2, index=19
                        // chan=3, index=28
                        // chan=4, index=37, out of bounds
                        // there are four channels (obivously, its a 4 port adapter)
                    },
                    Err(error) => println!("ERROR {}", error),
                }
            }
        }
    }
    // TODO safe reset on terminate?
}
