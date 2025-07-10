#include "../../../srcwrtimer/extshared/src/extension.h"
#include "../../../srcwrtimer/extshared/src/coreident.hpp"


extern const sp_nativeinfo_t DeltaNatives[];


void MyExtension::OnHandleDestroy(HandleType_t type, void* object) {}
bool MyExtension::GetHandleApproxSize(HandleType_t type, void* object, unsigned int* size) { return false; }


bool Extension_OnLoad(char* error, size_t maxlength)
{
	sharesys->AddNatives(myself, DeltaNatives);
	return true;
}

void Extension_OnUnload()
{
}

void Extension_OnAllLoaded() {}

static cell_t N_SRCWRDELTA_AsyncSaveReplay(IPluginContext* ctx, const cell_t* params)
{
	char *outbuf;
	(void)ctx->LocalToString(params[1], &outbuf);
	cell_t outbuflen = params[2];
	return 1;
}

extern const sp_nativeinfo_t DeltaNatives[] = {
	{"SRCWRDELTA_AsyncSaveReplay", N_SRCWRDELTA_AsyncSaveReplay},
	{NULL, NULL}
};
