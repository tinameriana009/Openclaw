#include "Modules/ModuleManager.h"

class FRuntimeTelemetryModule : public IModuleInterface
{
public:
    virtual void StartupModule() override
    {
    }

    virtual void ShutdownModule() override
    {
    }
};

IMPLEMENT_MODULE(FRuntimeTelemetryModule, RuntimeTelemetry)
