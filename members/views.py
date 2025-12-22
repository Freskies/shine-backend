from drf_spectacular.utils import extend_schema
from rest_framework.decorators import api_view
from rest_framework.response import Response
import members.models as member_models
import members.serializer as member_serializers


@extend_schema(
    responses=member_serializers.MemberSerializer(many=True),
    description="Retrieve a list of all members with their details and medical certificate status."
)
@api_view(['GET'])
def get_members(request):
    members = member_models.Member.objects.all()
    serializer = member_serializers.MemberSerializer(members, many=True)
    return Response(serializer.data)


@extend_schema(
    responses=member_serializers.MemberSerializer,
    description="Retrieve details of a specific member by their ID, including medical certificate status."
)
@api_view(['GET'])
def get_member(request, member_id):
    try:
        member = member_models.Member.objects.get(id=member_id)
    except member_models.Member.DoesNotExist:
        return Response({'error': 'Member not found'}, status=404)
    serializer = member_serializers.MemberSerializer(member)
    return Response(serializer.data)


@extend_schema(
    request=member_serializers.MemberSerializer,
    responses=member_serializers.MemberSerializer,
    description="Create a new member with the provided details."
)
@api_view(["POST"])
def create_member(request):
    serializer = member_serializers.MemberSerializer(data=request.data)
    if serializer.is_valid():
        serializer.save()
        return Response(serializer.data, status=201)
    return Response(serializer.errors, status=400)
