use super::*;
use crate::sample::demo_project;

fn configured() -> Project {
    let mut prj = demo_project();
    prj.plc.stations.push(Station { name: "CPU".into(), tag: "-A0".into(), device: "GEN.CPU-PN".into(), ..Default::default() });
    prj.plc.stations.push(Station { name: "ET1".into(), kind: StationKind::IoDevice, device: "GEN.IM-PN".into(), comment: "Remote <I/O>".into(), ..Default::default() });
    prj.plc_cards.insert("-A1".into(), PlcCard { station: "CPU".into(), ..Default::default() });
    prj.plc_cards.insert("-A2".into(), PlcCard { station: "ET1".into(), ..Default::default() });
    plc_hw::auto_configure(&mut prj, false);
    prj.plc.connections[0].cable = "-W10".into();
    prj
}

#[test]
fn export_has_devices_modules_subnets_and_tags() {
    let prj = configured();
    let text = export(&prj, &Netlist::build(&prj, true));
    let root = xml::parse(&text).unwrap();
    assert_eq!(root.attr("SchemaVersion"), Some("2.15"));
    let devices: Vec<&Element> = root.descendants("InternalElement").into_iter().filter(|e| role_of(e) == "Device").collect();
    assert_eq!(devices.iter().map(|d| d.attr("Name").unwrap()).collect::<Vec<_>>(), ["CPU", "ET1"]);
    let di = root.descendants("InternalElement").into_iter().find(|e| e.attr("Name") == Some("-A1")).unwrap();
    assert_eq!(number(di, "PositionNumber"), Some(2));
    assert_eq!(value(di, "TypeIdentifier").as_deref(), Some("OrderNumber:PLC-DI16"));
    assert_eq!(io_ranges(di), [(true, 0, 2)]);
    let subnet = root.descendants("InternalElement").into_iter().find(|e| role_of(e) == "Subnet").unwrap();
    assert_eq!(subnet.children("InternalLink").count(), 3, "two node links, one port link");
    let tags = root.descendants("InternalElement").into_iter().filter(|e| role_of(e) == "Tag").count();
    assert_eq!(tags, 5);
    assert!(text.contains("Remote &lt;I/O&gt;"));
}

#[test]
fn round_trip_into_an_empty_configuration() {
    let prj = configured();
    let text = export(&prj, &Netlist::build(&prj, true));
    let mut copy = demo_project();
    for pg in &mut copy.pages {
        for s in pg.symbols.iter_mut().filter(|s| s.tag == "-A1") {
            s.function_text = "old".into();
        }
    }
    let res = import(&mut copy, &text).unwrap();
    assert_eq!((res.stations, res.modules, res.networks, res.connections), (2, 2, 1, 1));
    assert_eq!(res.texts, 3, "texts of -A1 come back from the tag table");
    for st in &prj.plc.stations {
        assert_eq!(copy.plc.station(&st.name), Some(st), "{}", st.name);
    }
    assert_eq!(copy.plc.networks, prj.plc.networks);
    assert_eq!(copy.plc.connections, prj.plc.connections);
    for card in ["-A1", "-A2"] {
        let (a, b) = (&prj.plc_cards[card], &copy.plc_cards[card]);
        assert_eq!((&a.station, &a.rack, &a.slot, a.start_byte, a.output_byte), (&b.station, &b.rack, &b.slot, b.start_byte, b.output_byte));
    }
    assert!(copy.plc.descriptions.is_empty(), "all order numbers are known");
    assert!(plc_hw::problems(&copy).is_empty(), "{:?}", plc_hw::problems(&copy).iter().map(|i| &i.message).collect::<Vec<_>>());
}

#[test]
fn foreign_files_with_unknown_hardware() {
    // Layout of a TIA Portal export: rack as system type, ports as device items,
    // CAEX 2.15 links with `ID:Interface` references.
    let text = r#"<CAEXFile FileName="x.aml" SchemaVersion="2.15">
 <InstanceHierarchy Name="Project">
  <InternalElement ID="d1" Name="PLC_1">
   <InternalElement ID="r1" Name="Rack_0">
    <Attribute Name="TypeIdentifier"><Value>System:Rack.S71500</Value></Attribute>
    <InternalElement ID="c1" Name="PLC_1">
     <Attribute Name="PositionNumber"><Value>1</Value></Attribute>
     <Attribute Name="TypeIdentifier"><Value>OrderNumber:6ES7 515-2AM02-0AB0/V2.9</Value></Attribute>
     <InternalElement ID="if1" Name="PROFINET interface_1">
      <Attribute Name="NetworkAddress"><Value>192.168.0.10</Value></Attribute>
      <ExternalInterface ID="n1" Name="Node" RefBaseClassPath="AutomationProjectConfigurationInterfaceClassLib/LogicalEndPoint" />
      <InternalElement ID="p1" Name="Port_1">
       <ExternalInterface ID="pi1" Name="CommunicationPortInterface" RefBaseClassPath="CommunicationInterfaceClassLib/CommunicationPortInterface" />
      </InternalElement>
     </InternalElement>
    </InternalElement>
    <InternalElement ID="m1" Name="DI 32x24VDC HF_1">
     <Attribute Name="PositionNumber"><Value>2</Value></Attribute>
     <Attribute Name="TypeIdentifier"><Value>OrderNumber:6ES7 521-1BL00-0AB0/V2.2</Value></Attribute>
     <Attribute Name="Address"><Attribute Name="1">
      <Attribute Name="StartAddress"><Value>10</Value></Attribute><Attribute Name="Length"><Value>32</Value></Attribute><Attribute Name="IoType"><Value>Input</Value></Attribute>
     </Attribute></Attribute>
    </InternalElement>
   </InternalElement>
   <RoleRequirements RefBaseRoleClassPath="AutomationProjectConfigurationRoleClassLib/Device" />
  </InternalElement>
  <InternalElement ID="s1" Name="PN/IE_1">
   <Attribute Name="Type"><Value>Ethernet</Value></Attribute>
   <ExternalInterface ID="sn" Name="LogicalEndPoint" RefBaseClassPath="AutomationProjectConfigurationInterfaceClassLib/LogicalEndPoint" />
   <InternalLink Name="Link" RefPartnerSideA="if1:Node" RefPartnerSideB="s1:LogicalEndPoint" />
   <RoleRequirements RefBaseRoleClassPath="AutomationProjectConfigurationRoleClassLib/Subnet" />
  </InternalElement>
 </InstanceHierarchy>
</CAEXFile>"#;
    let mut prj = Project::default();
    let res = import(&mut prj, text).unwrap();
    assert_eq!((res.stations, res.modules, res.networks), (1, 1, 1));
    let st = &prj.plc.stations[0];
    assert_eq!((st.name.as_str(), st.kind, st.network.as_str(), st.address.as_str()), ("PLC_1", StationKind::Controller, "PN/IE_1", "192.168.0.10"));
    assert_eq!(st.device, "6ES7 515-2AM02-0AB0");
    let card = &prj.plc_cards["DI 32x24VDC HF_1"];
    assert_eq!((card.slot.as_str(), card.start_byte, card.device.as_str()), ("2", 10, "6ES7 521-1BL00-0AB0"));
    let di = plc_hw::find_description(&prj, &card.device).unwrap();
    assert_eq!((di.kind, di.inputs, di.input_bytes), (ModuleKind::DigitalInput, 32, 4));
    assert_eq!(plc_hw::find_description(&prj, &st.device).unwrap().ports[0].name, "Port_1");
    assert!(import(&mut prj, "<Root/>").is_err());
}
