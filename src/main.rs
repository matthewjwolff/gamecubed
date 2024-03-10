use evdev::uinput::{VirtualDevice, VirtualDeviceBuilder};
use evdev::{AbsInfo, AbsoluteAxisType, AttributeSet, EventType, InputEvent, Key, UinputAbsSetup};
use libusb::Context;
use std::time::Duration;

enum ControllerType {
    None,
    Wired,
    Wireless
}

// TODO only has ports 1-4
// TODO make const things const
fn make_new_controller(port_label: usize) -> VirtualDevice {
    // set up virtual pad
    let attr_set = {
        // keep the mutable ref inside this scope
        let mut attr_set = AttributeSet::<Key>::new();
        // four face buttons
        attr_set.insert(Key::BTN_NORTH);
        attr_set.insert(Key::BTN_SOUTH);
        attr_set.insert(Key::BTN_WEST);
        attr_set.insert(Key::BTN_EAST);

        // dpad
        // ok so xbox one controller maps dpad to "hat zero" axis (hardcoded 0 1)
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

    // xbox controller announced as 0,0,65535,255,4095,1
    let l_stick_x = UinputAbsSetup::new(AbsoluteAxisType::ABS_X, AbsInfo::new(128, 1, 255, 0, 0, 1)); 
    let l_stick_y = UinputAbsSetup::new(AbsoluteAxisType::ABS_Y, AbsInfo::new(128, 1, 255, 0, 0, 1)); 
    let r_stick_x = UinputAbsSetup::new(AbsoluteAxisType::ABS_Z, AbsInfo::new(128, 1, 255, 0, 0, 1)); 
    let r_stick_y = UinputAbsSetup::new(AbsoluteAxisType::ABS_RZ, AbsInfo::new(128, 1, 255, 0, 0, 1)); 
    let rt = UinputAbsSetup::new(AbsoluteAxisType::ABS_GAS, AbsInfo::new(30, 1, 255, 0, 0, 1)); 
    let lt = UinputAbsSetup::new(AbsoluteAxisType::ABS_BRAKE, AbsInfo::new(30, 1, 255, 0, 0, 1)); 
    
    VirtualDeviceBuilder::new()
    .unwrap()
    .name(&format!("Gamecube Controller Port {}",port_label))
    .with_keys(&attr_set)
    .unwrap()
    .with_absolute_axis(&l_stick_x).unwrap()
    .with_absolute_axis(&l_stick_y).unwrap()
    .with_absolute_axis(&r_stick_x).unwrap()
    .with_absolute_axis(&r_stick_y).unwrap()
    .with_absolute_axis(&rt).unwrap()
    .with_absolute_axis(&lt).unwrap()
        .build()
        .unwrap()
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
                            print!("{} ", i)
                        }
                        println!();
                        // from dolphin source code
                        // TODO more than one controller
                        //let i=0;
                        for i in 0..4 
                        {
                            // controller_payload_size = 9;
                            let controller_payload = &buffer[i*9+1..i*9+10];
                            let controller_type = controller_payload[0];
                            let plugged_in = (controller_type & 0b00010000)!=0;

                            // TODO actually use the enum (rust wants you to do pattern matching or something)
                            // TODO dropping a VirtualDevice will drop the underlying OwnedFd, which will close the file descriptor, which should remove the device. so it all works, as long as i can manually drop the value
                            let maybe_device = &mut devices[i];

                            if !plugged_in {
                                // take ownership of the optional (the old ref is now None)
                                // unconditionally drop it (if None, nothing happens, if Some, destructor is called)
                                drop(maybe_device.take())
                            } else {
                                // if Some, get it, if None, replace it with a new controller and return that
                                let v_device = maybe_device.get_or_insert_with(|| make_new_controller(i+1));

                                let buttons_1 = controller_payload[1];
                                let buttons_2 = controller_payload[2];
                            
                                // y axes need reflection about the center value (128)
                                let stick_x = controller_payload[3];
                                let stick_y = 128u8.wrapping_sub(controller_payload[4]).wrapping_sub(128);
                                let cstick_x = controller_payload[5];
                                let cstick_y = 128u8.wrapping_sub(controller_payload[6]).wrapping_sub(128);
                                let l = controller_payload[7];
                                let r = controller_payload[8];

                                // TODO can be optimized by using the result of lshift (i wonder if the complier does this already)
                                let a = (buttons_1 >> 0) & 1;
                                let b = (buttons_1 >> 1) & 1;
                                let x = (buttons_1 >> 2) & 1;
                                let y = (buttons_1 >> 3) & 1;
   
                                let dpad_left =  (buttons_1 >> 4) & 1;
                                let dpad_right = (buttons_1 >> 5) & 1;
                                let dpad_down =  (buttons_1 >> 6) & 1;
                                let dpad_up =    (buttons_1 >> 7) & 1;
                            
                                let start =   (buttons_2 >> 0) & 1;
                                let z =       (buttons_2 >> 1) & 1;
                                let r_click = (buttons_2 >> 2) & 1;
                                let l_click = (buttons_2 >> 3) & 1;
                                // i guess some unused bits in byte 3?

                                v_device.emit(&[
                                    InputEvent::new(EventType::KEY, Key::BTN_SOUTH.code(), a.into()),
                                    InputEvent::new(EventType::KEY, Key::BTN_EAST.code(), b.into()),
                                    InputEvent::new(EventType::KEY, Key::BTN_WEST.code(), x.into()),
                                    InputEvent::new(EventType::KEY, Key::BTN_NORTH.code(), y.into()),

                                    InputEvent::new(EventType::KEY, Key::BTN_DPAD_LEFT.code(), dpad_left.into()),
                                    InputEvent::new(EventType::KEY, Key::BTN_DPAD_RIGHT.code(), dpad_right.into()),
                                    InputEvent::new(EventType::KEY, Key::BTN_DPAD_DOWN.code(), dpad_down.into()),
                                    InputEvent::new(EventType::KEY, Key::BTN_DPAD_UP.code(), dpad_up.into()),
                                    
                                    InputEvent::new(EventType::KEY, Key::BTN_START.code(), start.into()),
                                    InputEvent::new(EventType::KEY, Key::BTN_SELECT.code(), z.into()),
                                    InputEvent::new(EventType::KEY, Key::BTN_TL.code(), l_click.into()),
                                    InputEvent::new(EventType::KEY, Key::BTN_TR.code(), r_click.into()),

                                    
                                    InputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_X.0, stick_x.into()),
                                    InputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_Y.0, stick_y.into()),
                                    InputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_Z.0, cstick_x.into()),
                                    InputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_RZ.0, cstick_y.into()),
                                    InputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_GAS.0, l.into()),
                                    InputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_BRAKE.0, r.into()),
                                    
                                ]).expect("Erorr pushing event");
                            }
                            

                            

                            
                        }
                         
                        // TODO what if size not 37?
                    },
                    Err(error) => println!("ERROR {}", error),
                }
            }
        }
    }
}
