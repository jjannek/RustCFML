<cfsavecontent variable="tagOut"><cfmodule template="callsvc.cfm"></cfsavecontent><cfoutput>page=#application.svc.ping()#;tag=#trim(tagOut)#;</cfoutput>
