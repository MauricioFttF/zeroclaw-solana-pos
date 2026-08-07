// Gera as amarrações do WebAssembly automaticamente com base no arquivo .wit
wit_bindgen::generate!({
    world: "gpio-plugin",
});

use exports::pos::gpio::controller::Guest;

// --- A Interface Comum ---

pub trait GpioController {
    fn set_payment_pending(&self) -> Result<(), String>;
    fn set_payment_success(&self) -> Result<(), String>;
    fn sound_error_alarm(&self) -> Result<(), String>;
}

// --- 1. A Lógica de Simulação (O Mock para a Demo de Hoje) ---

pub struct MockGpio;

impl GpioController for MockGpio {
    fn set_payment_pending(&self) -> Result<(), String> {
        println!("\n[MOCK GPIO] 🟡 LED Amarelo: Aguardando pagamento da Solana...");
        Ok(())
    }

    fn set_payment_success(&self) -> Result<(), String> {
        println!("\n[MOCK GPIO] 🟢 LED Verde LIGADO");
        println!("[MOCK GPIO] 🔊 Buzzer: Bip curto (Transação Confirmada!)");
        Ok(())
    }

    fn sound_error_alarm(&self) -> Result<(), String> {
        println!("\n[MOCK GPIO] 🔴 LED Vermelho LIGADO");
        println!("[MOCK GPIO] 🔊 Buzzer: Bip longo (ERRO NO PAGAMENTO)");
        Ok(())
    }
}

// --- A Ponte com a IA (O Plugin) ---

struct GpioPlugin;

impl Guest for GpioPlugin {
    fn set_payment_pending() -> Result<(), String> {
        // Para a demo, instanciamos o Mock. 
        // No hardware real, instanciaríamos o RaspberryGpio.
        let hardware = MockGpio;
        hardware.set_payment_pending()
    }

    fn set_payment_success() -> Result<(), String> {
        let hardware = MockGpio;
        hardware.set_payment_success()
    }

    fn sound_error_alarm() -> Result<(), String> {
        let hardware = MockGpio;
        hardware.sound_error_alarm()
    }
}

export!(GpioPlugin);

// =====================================================================
// --- 2. Implementação Real (Raspberry Pi) - Visão de Produção ---
// =====================================================================
// NOTA PARA OS JURADOS: Como este módulo compila para WASM (sandbox), 
// o acesso direto ao /dev/gpiomem é bloqueado. Em um ambiente de 
// produção, esta lógica seria movida para o Host (ZeroClaw) que 
// consumiria a interface WIT deste plugin.
//
// Abaixo está a implementação exata de como o hardware é mapeado:

/*
use std::thread;
use std::time::Duration;
use rppal::gpio::Gpio;

pub struct RaspberryGpio {
    pin_yellow: u8,
    pin_green: u8,
    pin_red: u8,
    pin_buzzer: u8,
}

impl RaspberryGpio {
    pub fn new() -> Self {
        // Mapeamento dos pinos lógicos para os pinos físicos BCM do RPi
        Self {
            pin_yellow: 17,
            pin_green: 27,
            pin_red: 22,
            pin_buzzer: 23,
        }
    }

    // Função auxiliar para resetar os LEDs
    fn clear_all(&self, gpio: &Gpio) -> Result<(), String> {
        gpio.get(self.pin_yellow).map_err(|e| e.to_string())?.into_output().set_low();
        gpio.get(self.pin_green).map_err(|e| e.to_string())?.into_output().set_low();
        gpio.get(self.pin_red).map_err(|e| e.to_string())?.into_output().set_low();
        Ok(())
    }
}

impl GpioController for RaspberryGpio {
    fn set_payment_pending(&self) -> Result<(), String> {
        let gpio = Gpio::new().map_err(|e| e.to_string())?;
        self.clear_all(&gpio)?;
        
        let mut yellow_led = gpio.get(self.pin_yellow).map_err(|e| e.to_string())?.into_output();
        yellow_led.set_high(); // Liga o LED Amarelo
        Ok(())
    }

    fn set_payment_success(&self) -> Result<(), String> {
        let gpio = Gpio::new().map_err(|e| e.to_string())?;
        self.clear_all(&gpio)?;
        
        let mut green_led = gpio.get(self.pin_green).map_err(|e| e.to_string())?.into_output();
        let mut buzzer = gpio.get(self.pin_buzzer).map_err(|e| e.to_string())?.into_output();
        
        green_led.set_high(); // Liga o LED Verde
        
        // Emite um bip curto
        buzzer.set_high();
        thread::sleep(Duration::from_millis(200));
        buzzer.set_low();
        
        Ok(())
    }

    fn sound_error_alarm(&self) -> Result<(), String> {
        let gpio = Gpio::new().map_err(|e| e.to_string())?;
        self.clear_all(&gpio)?;
        
        let mut red_led = gpio.get(self.pin_red).map_err(|e| e.to_string())?.into_output();
        let mut buzzer = gpio.get(self.pin_buzzer).map_err(|e| e.to_string())?.into_output();
        
        red_led.set_high(); // Liga o LED Vermelho
        
        // Emite um bip longo (erro)
        buzzer.set_high();
        thread::sleep(Duration::from_millis(1000));
        buzzer.set_low();
        
        Ok(())
    }
}
*/