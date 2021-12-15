use clap::arg_enum;

arg_enum!{
    #[derive(PartialEq, Debug, Clone)]
    pub enum DeviceAction{
        Get,
    }
}

